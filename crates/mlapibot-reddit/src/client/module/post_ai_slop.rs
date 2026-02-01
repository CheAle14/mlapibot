use std::{
    collections::{BinaryHeap, HashMap, HashSet, VecDeque},
    ops::{ControlFlow, Range},
};

use annotate_snippets::{AnnotationKind, Group, Level, Origin, Renderer, Snippet};
use anyhow::Context;
use bumpalo::Bump;
use futures_util::TryStreamExt;
use markdown::{mdast::Node, unist::Position};
use mlapibot_analysis::{Url, extract_all_links};
use mlapibot_common::{DataMetaData, RunningStat};
use octocrab::{Octocrab, models::repos::RepoCommit, repos::RepoHandler};

use crate::client::module::impl_mask_subreddits;

type BVec<'arena, T> = bumpalo::collections::Vec<'arena, T>;

pub struct PostAiSlop {
    seen: HashSet<RepoLink>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum RepoWebsite {
    GitHub,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct RepoLink {
    website: RepoWebsite,
    repo: String,
}

impl std::fmt::Display for RepoLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.website {
            RepoWebsite::GitHub => f.write_str("https://github.com/")?,
        }

        f.write_str(&self.repo)
    }
}

impl RepoLink {
    #[cfg(test)]
    pub fn parse(link: &str) -> Self {
        let url = Url::parse(link).unwrap();
        Self::new(&url).unwrap()
    }

    pub fn new(link: &Url) -> anyhow::Result<Self> {
        let website = match link.domain() {
            "github.com" => RepoWebsite::GitHub,
            other => anyhow::bail!("only github.com supported, was: {other:?}"),
        };

        let mut iter = link
            .path()
            .strip_prefix('/')
            .unwrap_or_else(|| link.path())
            .split('/');

        let repo_owner = iter
            .next()
            .ok_or_else(|| anyhow::anyhow!("no repo owner: {}", link.path()))?;

        let repo_name = iter
            .next()
            .ok_or_else(|| anyhow::anyhow!("no repo name: {}", link.path()))?
            .trim_suffix(".git");

        Ok(Self {
            website,
            repo: format!("{repo_owner}/{repo_name}"),
        })
    }

    pub fn owner_and_name(&self) -> (&str, &str) {
        unsafe { self.repo.split_once('/').unwrap_unchecked() }
    }
}

#[async_trait::async_trait(?Send)]
impl super::Module for PostAiSlop {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            seen: HashSet::new(),
        }
    }

    fn name(&self) -> &'static str {
        "post_ai_slop"
    }

    fn wants(&self) -> super::ModuleWants {
        super::ModuleWants::POSTS
    }

    impl_mask_subreddits!(ai_slop => posts);

    async fn run_post<'client>(
        &mut self,
        client: &mut crate::client::ModuleRedditClient<'client>,
        _subreddit: &mut crate::client::Subreddit,
        config: Option<&crate::config::SubredditConfig>,
        post: &crate::Submission,
        has_seen: bool,
    ) -> anyhow::Result<super::PostAction> {
        if has_seen {
            return Ok(super::PostAction::Ignore);
        }

        let Some(slopconf) = config.and_then(|v| v.ai_slop.as_ref()) else {
            return Ok(super::PostAction::Ignore);
        };

        let Some(github) = client.github else {
            return Ok(super::PostAction::Ignore);
        };

        let mut links = HashSet::new();
        if let Some(Ok(url)) = post.url().as_ref().map(|v| Url::parse(&v)) {
            if let Ok(url) = RepoLink::new(&url) {
                links.insert(url);
            }
        }

        for link in extract_all_links(post.selftext(), None) {
            if let Ok(link) = RepoLink::new(&link) {
                links.insert(link);
            }
        }

        let bump = Bump::new();

        for link in links {
            if !self.seen.insert(link.clone()) {
                continue;
            }

            match determine_ai_slop(&*github, &link, &bump).await {
                Ok(slop) => {
                    let subject = format!("Slop report for {}", link);
                    let mut body = format!("For {}  \n{slop:?}", post.permalink());

                    let renderer = Renderer::plain()
                        .decor_style(annotate_snippets::renderer::DecorStyle::Unicode);

                    body.push_str("\n\n```");

                    for report in slop.reports {
                        let r = renderer.render(&report);
                        body.push('\n');
                        body.push_str(&r);
                    }

                    body.push_str("\n\n```");

                    client
                        .client
                        .compose_message(&slopconf.modmail_to, &subject, &body)
                        .await?;
                }
                Err(err) => {
                    eprintln!("{link}: {err}")
                }
            }
        }

        Ok(super::PostAction::Ignore)
    }
}

struct Perc(f32);

impl std::fmt::Display for Perc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = self.0 * 100.0;
        write!(f, "{v:.1}%")
    }
}

impl std::fmt::Debug for Perc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = self.0 * 100.0;
        write!(f, "{v:.1}%")
    }
}

#[derive(Default, PartialEq)]
struct Ratio {
    num: u32,
    total: u32,
}

impl Ratio {
    pub fn ratio(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }

        self.num as f32 / self.total as f32
    }
}

impl std::fmt::Debug for Ratio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ratio")
            .field("n", &self.num)
            .field("t", &self.total)
            .field("r", &Perc(self.ratio()))
            .finish()
    }
}

#[derive(Debug, Default, PartialEq)]
struct ReadmeSlopness {
    /// The % of list items that contain an emoji
    emoji_points: Ratio,
    /// The % of headings that contain an emoji
    emoji_headings: Ratio,
    /// The total number of emoji
    num_emoji: u32,
    /// The total number of em—dashes
    num_em_dash: u32,
    /// The total number of characters in the text
    total_chars: u32,
}

struct Slopness<'arena> {
    readme: ReadmeSlopness,
    commits: CommitsSlopness,
    // The emitted diagnostics
    reports: Vec<BVec<'arena, Group<'arena>>>,
}

impl<'arena> std::fmt::Debug for Slopness<'arena> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Slopness")
            .field("readme", &self.readme)
            .field("commits", &self.commits)
            .field("reports", &self.reports.len())
            .finish()
    }
}

trait GitClient: Sized {
    type Repository<'l>: GitRepository<Self>
    where
        Self: 'l;

    async fn open_repo<'l>(
        &'l self,
        owner: &str,
        name: &str,
    ) -> anyhow::Result<Self::Repository<'l>>;
}

impl GitClient for Octocrab {
    type Repository<'l> = octocrab::repos::RepoHandler<'l>;

    async fn open_repo<'l>(
        &'l self,
        owner: &str,
        name: &str,
    ) -> anyhow::Result<Self::Repository<'l>> {
        println!("open_repo: {owner} / {name}");
        let repo = self.repos(owner, name);
        Ok(repo)
    }
}

struct RepoContent {
    pub name: String,
    pub decoded: Option<String>,
}

trait GitRepository<C: GitClient> {
    type Commit: GitCommit;

    async fn fetch_content(&self, path: &str) -> anyhow::Result<Vec<RepoContent>>;

    async fn fetch_file(&self, path: &str) -> anyhow::Result<Option<String>> {
        let mut content = self.fetch_content(path).await?;
        if content.len() != 1 {
            anyhow::bail!("no file at {path}");
        }

        Ok(content.pop().and_then(|v| v.decoded))
    }

    async fn fetch_readme(&self) -> anyhow::Result<String>;

    async fn for_each_commit<F>(&self, client: &C, callback: F) -> anyhow::Result<()>
    where
        F: FnMut(Self::Commit) -> ControlFlow<()>;
}

impl<'l> GitRepository<Octocrab> for RepoHandler<'l> {
    type Commit = octocrab::models::repos::RepoCommit;

    async fn fetch_content(&self, path: &str) -> anyhow::Result<Vec<RepoContent>> {
        println!("fetch {path}");
        let content = self.get_content().path(path).send().await?;

        Ok(content
            .items
            .into_iter()
            .map(|v| RepoContent {
                decoded: v.decoded_content(),
                name: v.name,
            })
            .collect())
    }

    async fn fetch_readme(&self) -> anyhow::Result<String> {
        // println!("fetch readme");
        let content = self.get_readme().send().await.context("fetch readme")?;
        Ok(content.decoded_content().unwrap_or_default())
    }

    async fn for_each_commit<F>(&self, client: &Octocrab, mut callback: F) -> anyhow::Result<()>
    where
        F: FnMut(Self::Commit) -> ControlFlow<()>,
    {
        // println!("for-each-commit");
        let s = self
            .list_commits()
            .per_page(100)
            .send()
            .await?
            .into_stream(client);

        tokio::pin!(s);

        while let Some(cmm) = s.try_next().await? {
            match callback(cmm) {
                ControlFlow::Continue(()) => continue,
                ControlFlow::Break(()) => break,
            }
        }

        Ok(())
    }
}

type DateTimeUtc = chrono::DateTime<chrono::Utc>;

trait GitCommit {
    fn sha(&self) -> &str;
    fn message(&self) -> &str;
    fn date(&self) -> Option<DateTimeUtc>;

    // A hopefully stable identifier for the author of this commit
    //
    // This may be their account username, or potentially an email.
    fn author_identifier(&self) -> Option<&str>;
}

impl GitCommit for RepoCommit {
    fn sha(&self) -> &str {
        &self.sha
    }

    fn message(&self) -> &str {
        &self.commit.message
    }

    fn date(&self) -> Option<DateTimeUtc> {
        self.commit
            .committer
            .as_ref()
            .and_then(|v| v.date)
            .or_else(|| self.commit.author.as_ref().and_then(|v| v.date))
    }

    fn author_identifier(&self) -> Option<&str> {
        if let Some(author) = self.author.as_ref() {
            return Some(author.login.as_str());
        }

        if let Some(user) = self.commit.author.as_ref() {
            if let Some(email) = user.email.as_ref() {
                return Some(email.as_str());
            }
            return Some(user.name.as_str());
        }

        if let Some(user) = self.commit.committer.as_ref() {
            if let Some(email) = user.email.as_ref() {
                return Some(email.as_str());
            }
            return Some(user.name.as_str());
        }

        None
    }
}

async fn determine_ai_slop<'arena, C: GitClient>(
    client: &C,
    link: &RepoLink,
    arena: &'arena Bump,
) -> anyhow::Result<Slopness<'arena>> {
    let (repo_owner, repo_name) = link.owner_and_name();
    let repo = client.open_repo(repo_owner, repo_name).await?;

    let readme = repo.fetch_readme().await?;
    let readme = &*arena.alloc_str(&readme);

    let mut reports = Vec::new();
    let readme = guess_readme_slop(arena, &mut reports, &readme)?;
    let commits = guess_commit_slop::<C>(arena, &mut reports, client, &repo).await?;

    guess_files_slop::<C>(arena, &mut reports, &repo).await?;

    Ok(Slopness {
        readme,
        commits,
        reports,
    })
}

#[derive(Debug)]
struct CommitsSlopness {
    ai_co_author: Ratio,
    /// Commits per day for each contributor
    commits_per_day: DataMetaData,
}

async fn guess_commit_slop<'arena, 'git, C: GitClient>(
    arena: &'arena Bump,
    reports: &mut Vec<BVec<'arena, Group<'arena>>>,
    client: &C,
    repo: &C::Repository<'git>,
) -> anyhow::Result<CommitsSlopness> {
    let mut ai_co_author = Ratio::default();
    let mut ai_co_author_snippets = Vec::new();

    struct ContributorStats {
        total_commits: u32,
        oldest: Option<DateTimeUtc>,
    }

    impl ContributorStats {
        pub fn commits_per_day(&self, now: DateTimeUtc) -> Option<f32> {
            if self.total_commits < 10 {
                return None;
            }

            let oldest = self.oldest?;
            let secs = now.signed_duration_since(oldest).as_seconds_f32();
            let mins = secs / 60.0;
            let hours = mins / 60.0;
            let days = hours / 24.0;

            Some(self.total_commits as f32 / days)
        }
    }

    let mut contributors: HashMap<String, ContributorStats> = HashMap::new();

    repo.for_each_commit(client, |commit| {
        ai_co_author.total += 1;

        if let Some(id) = commit.author_identifier() {
            let date = commit.date();

            contributors
                .entry(id.to_owned())
                .and_modify(|s| {
                    s.total_commits += 1;

                    if let Some(this) = date
                        && let Some(eldest) = s.oldest.as_mut()
                        && this < *eldest
                    {
                        *eldest = this;
                    }
                })
                .or_insert_with(|| ContributorStats {
                    total_commits: 1,
                    oldest: date,
                });
        }

        if let Some(idx) = commit.message().find("Co-Authored-By: Claude") {
            let span = idx..(idx + "Co-Authored-By: Claude".len());
            ai_co_author.num += 1;

            let text = &*arena.alloc_str(commit.message());
            let sha = &*arena.alloc_str(commit.sha());

            ai_co_author_snippets.push(
                Snippet::source(text)
                    .annotation(AnnotationKind::Primary.span(span))
                    .path(sha),
            )
        }

        if ai_co_author.total >= 1000 {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    })
    .await?;

    let mut commits_per_day = RunningStat::default();
    let now = chrono::Utc::now();
    for (_id, contrib) in contributors {
        if let Some(cpd) = contrib.commits_per_day(now) {
            commits_per_day += cpd;
        }
    }
    let commits_per_day = commits_per_day.results();

    if commits_per_day.max > 30.0 {
        reports.push(bumpalo::vec![in arena;
            Group::with_title(Level::ERROR
                .primary_title(format!(
                    "very high commit rate: {:.1} per day",
                    commits_per_day.max
                ))),
        ]);
    } else if commits_per_day.max > 15.0 {
        reports.push(bumpalo::vec![in arena;
            Group::with_title(Level::WARNING
                .primary_title(format!(
                    "high commit rate: {:.1} per day",
                    commits_per_day.max
                )))
        ]);
    }

    if ai_co_author.ratio() > 0.5 {
        reports.push(bumpalo::vec![in arena; Level::ERROR
            .primary_title(format!(
                "{} commits have agentic co-authors",
                Perc(ai_co_author.ratio())
            ))
            .elements(ai_co_author_snippets.into_iter().take(10))]);
    } else if ai_co_author.ratio() > 0.25 {
        reports.push(bumpalo::vec![in arena; Level::WARNING
            .primary_title(format!(
                "{} commits have agentic co-authors",
                Perc(ai_co_author.ratio())
            ))
            .elements(ai_co_author_snippets.into_iter().take(10))]);
    }

    Ok(CommitsSlopness {
        ai_co_author,
        commits_per_day,
    })
}

async fn guess_files_slop<'arena, 'git, C>(
    arena: &'arena Bump,
    reports: &mut Vec<BVec<'arena, Group<'arena>>>,
    repo: &C::Repository<'git>,
) -> anyhow::Result<()>
where
    C: GitClient,
{
    let root_files = repo.fetch_content("").await?;

    if root_files.iter().any(|v| v.name == ".gitignore")
        && let Some(gitignore) = repo.fetch_file(".gitignore").await?
    {
        let gitignore = &*arena.alloc_str(&gitignore);

        let mut annotations = Vec::new();
        let mut offset = 0;

        for line in gitignore.split_inclusive('\n') {
            let line_span = offset..(offset + line.len() - 1);

            for phrase in ["CLAUDE.md", "GEMINI.md", "AGENTS.md", ".claude", ".serena"] {
                if line.starts_with(phrase) {
                    annotations.push(
                        Snippet::source(gitignore)
                            .annotation(
                                AnnotationKind::Context
                                    .span(line_span.clone())
                                    .label("ignores agentic file or directory"),
                            )
                            .path(".gitignore"),
                    );
                }
            }

            offset += line.len();
        }

        if annotations.len() > 0 {
            let mut report = BVec::new_in(arena);

            report.push(
                Level::ERROR
                    .primary_title("files contain AI-related references")
                    .elements(annotations.into_iter().take(10)),
            );

            reports.push(report);
        }
    }

    for agentic in ["CLAUDE.md", "GEMINI.md", "AGENTS.md", ".claude"] {
        if root_files.iter().any(|v| v.name == agentic) {
            let mut report = BVec::new_in(arena);
            report.push(
                Level::ERROR
                    .primary_title("agentic instruction file or directory detected")
                    .element(Origin::path(agentic)),
            );
            reports.push(report);
        }
    }

    Ok(())
}

fn guess_readme_slop<'arena>(
    arena: &'arena Bump,
    reports: &mut Vec<BVec<'arena, Group<'arena>>>,
    readme: &'arena str,
) -> anyhow::Result<ReadmeSlopness> {
    let mut slopness = ReadmeSlopness::default();
    let mut seen_emoji_in_lists = HashSet::new();

    let mut this_report = BVec::<'arena, Group<'arena>>::new_in(arena);

    let mut pending_em_dashes = Vec::new();
    let mut pending_emoji_headings = Vec::new();
    let mut pending_emoji_points = Vec::new();

    let ast = markdown::to_mdast(readme, &markdown::ParseOptions::gfm()).map_err(|e| {
        anyhow::anyhow!(
            "failed to parse: {} (rule_id={}, source={}, place={:?})",
            e.reason,
            e.rule_id,
            e.source,
            e.place
        )
    })?;

    let _ = std::fs::write(r"D:\_GitHub\mlapibot\ast.txt", format!("{ast:#?}"));

    #[derive(Debug, Clone)]
    enum WalkParent {
        List { span: Range<usize> },
        Heading { span: Range<usize> },
    }

    struct WalkCtx {
        depth: usize,
        order: usize,
        parent: Option<WalkParent>,
        node: markdown::mdast::Node,
    }

    impl std::fmt::Debug for WalkCtx {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let node = format!("{:?}", self.node);

            let (start, _rest) = node.split_once('{').unwrap_or_else(|| (node.as_str(), ""));

            f.debug_struct("WalkCtx")
                .field("depth", &self.depth)
                .field("order", &self.order)
                .field("parent", &self.parent)
                .field("node", &start)
                .finish()
        }
    }

    impl PartialEq for WalkCtx {
        fn eq(&self, other: &Self) -> bool {
            self.depth == other.depth && self.order == other.order
        }
    }

    impl Eq for WalkCtx {}

    impl PartialOrd for WalkCtx {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(&other))
        }
    }

    impl Ord for WalkCtx {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            match self.depth.cmp(&other.depth) {
                std::cmp::Ordering::Equal => other.order.cmp(&self.order),
                other => other,
            }
        }
    }

    #[derive(Debug, Clone)]
    struct SpanString {
        text: String,
        span: Range<usize>,
        walk_depth: usize,
    }

    let mut last_heading: Option<SpanString> = None;

    let mut queue = BinaryHeap::new();
    queue.push(WalkCtx {
        depth: 0,
        order: 0,
        node: ast,
        parent: None,
    });

    while let Some(ctx) = queue.pop() {
        macro_rules! push_children {
            ($item:ident, !) => {
                for (order, node) in $item.children.into_iter().enumerate() {
                    queue.push(WalkCtx {
                        depth: ctx.depth + 1,
                        order,
                        parent: None,
                        node,
                    })
                }
            };
            ($item:ident, $parent:expr) => {
                for (order, node) in $item.children.into_iter().enumerate() {
                    queue.push(WalkCtx {
                        depth: ctx.depth + 1,
                        order,
                        parent: Some($parent.clone()),
                        node,
                    })
                }
            };
            ($item:ident) => {
                for (order, node) in $item.children.into_iter().enumerate() {
                    queue.push(WalkCtx {
                        depth: ctx.depth + 1,
                        order,
                        parent: ctx.parent.clone(),
                        node,
                    })
                }
            };
        }

        match ctx.node {
            Node::Text(text) => {
                if ctx
                    .parent
                    .as_ref()
                    .is_some_and(|p| matches!(p, WalkParent::Heading { .. }))
                    && let Some(heading) = last_heading.as_mut()
                {
                    let lowercase = text.value.to_lowercase();
                    heading.text.push_str(&lowercase);
                }

                let mut saw_emoji = false;
                for (idx, chr) in text.value.char_indices() {
                    slopness.total_chars += 1;
                    if chr == '—' {
                        let start = text.position.as_ref().unwrap().start.offset + idx;
                        pending_em_dashes.push((start, start + 1));
                    } else if is_char_emoji(chr) {
                        slopness.num_emoji += 1;

                        if saw_emoji {
                            continue;
                        }

                        saw_emoji = true;
                        match ctx.parent {
                            Some(WalkParent::Heading { ref span }) => {
                                slopness.emoji_headings.num += 1;
                                pending_emoji_headings.push(
                                    Snippet::source(readme)
                                        .annotation(AnnotationKind::Context.span(span.clone())),
                                );
                            }
                            Some(WalkParent::List { ref span })
                                if seen_emoji_in_lists.insert(span.clone()) =>
                            {
                                slopness.emoji_points.num += 1;
                                pending_emoji_points.push(
                                    Snippet::source(readme)
                                        .annotation(AnnotationKind::Context.span(span.clone())),
                                );
                            }
                            _ => (),
                        }
                    }
                }
            }
            Node::Root(root) => push_children!(root),
            Node::Heading(heading) => {
                slopness.emoji_headings.total += 1;
                let span = heading.into_span();

                last_heading = Some(SpanString {
                    span: span.clone(),
                    walk_depth: ctx.depth,
                    text: String::new(),
                });

                push_children!(heading, WalkParent::Heading { span: span.clone() });
            }
            Node::ListItem(list) => {
                slopness.emoji_points.total += 1;

                let span = list.into_span();
                push_children!(list, WalkParent::List { span: span.clone() })
            }

            Node::List(heading) => push_children!(heading),
            Node::Paragraph(v) => push_children!(v),
            Node::FootnoteDefinition(v) => push_children!(v),
            Node::MdxJsxFlowElement(v) => push_children!(v),
            Node::Delete(v) => push_children!(v),
            Node::Emphasis(v) => push_children!(v),
            Node::MdxJsxTextElement(v) => push_children!(v),
            Node::Link(v) => push_children!(v),
            Node::LinkReference(v) => push_children!(v),
            Node::Strong(v) => push_children!(v),

            // Stop propagating the list/heading into these
            // complex formatting sections.
            Node::Blockquote(v) => push_children!(v, !),
            Node::Table(v) => push_children!(v, !),
            Node::TableRow(v) => push_children!(v, !),
            Node::TableCell(v) => push_children!(v, !),

            Node::Code(code) => {
                println!("code {:?} @ {:?}", code.lang, code.position);
                println!(" under: {:?}", last_heading);

                if let Some(heading) = last_heading.as_ref()
                    && heading.text.contains("architecture")
                    && code.lang.is_some_and(|lang| lang == "mermaid")
                {
                    let span = code.position.into_span();
                    // since the diagram might be quite large, we only want
                    // to highlight the start of it.
                    let span = span.start..std::cmp::min(span.end, span.start + "```mermaid".len());

                    this_report.push(
                        Level::ERROR
                            .primary_title("possible AI-generated architecture diagram")
                            .element(Snippet::source(readme).annotation(
                                AnnotationKind::Primary.span(span).label("diagram here"),
                            ))
                            .element(
                                Snippet::source(readme).annotation(
                                    AnnotationKind::Context
                                        .span(heading.span.clone())
                                        .label("underneath architecture heading here"),
                                ),
                            ),
                    )
                }
            }

            Node::MdxjsEsm(_)
            | Node::Toml(_)
            | Node::Yaml(_)
            | Node::Break(_)
            | Node::InlineCode(_)
            | Node::InlineMath(_)
            | Node::MdxTextExpression(_)
            | Node::FootnoteReference(_)
            | Node::Html(_)
            | Node::Image(_)
            | Node::ImageReference(_)
            | Node::Math(_)
            | Node::MdxFlowExpression(_)
            | Node::ThematicBreak(_)
            | Node::Definition(_) => (),
        }
    }

    if slopness.emoji_headings.ratio() > 0.5 {
        this_report.push(
            Level::ERROR
                .primary_title(format!(
                    "{} headings use emoji",
                    Perc(slopness.emoji_headings.ratio())
                ))
                .elements(pending_emoji_headings.into_iter().take(10)),
        );
    } else if slopness.emoji_headings.ratio() > 0.25 {
        this_report.push(
            Level::WARNING
                .primary_title(format!(
                    "{} headings use emoji",
                    Perc(slopness.emoji_headings.ratio())
                ))
                .elements(pending_emoji_headings.into_iter().take(10)),
        );
    }

    if slopness.emoji_points.ratio() > 0.25 {
        this_report.push(
            Level::ERROR
                .primary_title(format!(
                    "{} list entries use emoji",
                    Perc(slopness.emoji_points.ratio())
                ))
                .elements(pending_emoji_points),
        );
    } else if slopness.emoji_points.ratio() > 0.1 {
        this_report.push(
            Level::WARNING
                .primary_title(format!(
                    "{} list entries use emoji",
                    Perc(slopness.emoji_points.ratio())
                ))
                .elements(pending_emoji_points.into_iter().take(10)),
        );
    }

    if this_report.len() > 0 {
        reports.push(this_report);
    }

    Ok(slopness)
}

trait IntoSpan {
    fn into_span(&self) -> Range<usize>;
}

impl IntoSpan for Option<Position> {
    fn into_span(&self) -> Range<usize> {
        match self {
            Some(pos) => pos.start.offset..pos.end.offset,
            None => 0..0,
        }
    }
}

macro_rules! delegate_into_span {
    ($($item:ident),* $(,)?) => {
        $(
            impl IntoSpan for markdown::mdast::$item {
                fn into_span(&self) -> Range<usize> {
                    self.position.into_span()
                }
            }
        )*
    };
}

delegate_into_span!(Text, Heading, List, ListItem);

// this doesn't correctly handle multi-char emoji.
// e.g., flags that visually render as one item will be counted twice
fn is_char_emoji(chr: char) -> bool {
    if chr.is_ascii() {
        return false;
    }

    unic_emoji_char::is_emoji(chr)
}

#[cfg(test)]
mod tests {
    use annotate_snippets::Renderer;
    use bumpalo::Bump;

    use crate::client::module::post_ai_slop::{Ratio, ReadmeSlopness, RepoLink};

    #[test]
    fn our_readme_is_not_slop() {
        static THIS_README: &str = include_str!("../../../../../README.md");

        let arena = Bump::new();
        let mut reports = Vec::new();

        let slop = super::guess_readme_slop(&arena, &mut reports, THIS_README).unwrap();

        assert_eq!(reports.len(), 0);
        assert_eq!(slop.num_em_dash, 0);
        assert_eq!(slop.num_emoji, 0);
        assert_eq!(slop.emoji_points.ratio(), 0.0);
        assert_eq!(slop.emoji_headings.ratio(), 0.0);
    }

    #[tokio::test]
    async fn rustc_is_not_slop() {
        println!("init");

        let octo = octocrab::instance();
        let url = RepoLink::parse("https://github.com/rust-lang/rust");
        println!("determine");

        let arena = Bump::new();

        let slopness = super::determine_ai_slop(octo.as_ref(), &url, &arena)
            .await
            .unwrap();

        println!("{slopness:?}");
    }

    #[tokio::test]
    async fn get_repo_slopness() {
        println!("init");

        // ai use:
        // https://github.com/mattdef/dampen
        // https://github.com/cdump/proton-tui
        // https://github.com/cledouarec/sara
        // https://github.com/colliery-io/plissken
        // https://github.com/Daemoniorum-LLC/arcanum
        // https://github.com/samvallad33/vestige
        // https://github.com/YeautyYE/skill-rust-ffmpeg
        //
        // ???:
        // https://github.com/landaire/stoptrackingme
        let octo = octocrab::instance();
        let url = RepoLink::parse("https://github.com/YeautyYE/skill-rust-ffmpeg");
        println!("determine");

        let arena = Bump::new();

        let slopness = super::determine_ai_slop(octo.as_ref(), &url, &arena)
            .await
            .unwrap();

        let renderer = Renderer::styled();

        let mut total_len = 0;
        for report in &slopness.reports {
            let text = renderer.render(&report);
            total_len += text.len();
            println!("\n\n{text}");
        }

        println!("length: {total_len}");

        println!("{slopness:#?}");
    }

    #[test]
    fn fake_readme_is_slop() {
        /// Generated by ChatGPT, naturally.
        static SLOP_README: &str = include_str!("slop_readme.md");

        let arena = Bump::new();
        let mut reports = Vec::new();
        let slop = super::guess_readme_slop(&arena, &mut reports, SLOP_README).unwrap();

        let renderer = Renderer::styled();

        let mut total_len = 0;
        for report in &reports {
            let text = renderer.render(&report);
            total_len += text.len();
            println!("\n\n{text}");
        }

        println!("length: {total_len}");

        assert_eq!(
            slop,
            ReadmeSlopness {
                emoji_points: Ratio { num: 9, total: 33 },
                emoji_headings: Ratio { num: 12, total: 12 },
                num_emoji: 49,
                num_em_dash: 0,
                total_chars: 2608
            }
        );
    }

    #[test]
    fn does_not_count_text_as_emoji() {
        for chr in "hello world123;'#[]'“', '”'".chars() {
            assert!(!super::is_char_emoji(chr), "{chr:?}");
        }
    }

    #[test]
    fn counts_emoji_as_emoji() {
        assert!(super::is_char_emoji('✅'));
        assert!(super::is_char_emoji('✔'));
    }

    #[test]
    fn does_not_count_math_symbols_as_emoji() {
        assert!(!super::is_char_emoji('≤'));
        assert!(!super::is_char_emoji('±'));
        assert!(!super::is_char_emoji('×'));
    }
}
