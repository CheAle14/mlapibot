use std::{
    collections::{HashSet, VecDeque},
    ops::{ControlFlow, Range},
};

use annotate_snippets::{AnnotationKind, Group, Level, Origin, Renderer, Report, Snippet};
use anyhow::Context;
use bumpalo::Bump;
use futures_util::TryStreamExt;
use markdown::{
    mdast::Node,
    unist::{Point, Position},
};
use mlapibot_analysis::{Url, extract_all_links};
use octocrab::{GitHubError, Octocrab, models::repos::RepoCommit, repos::RepoHandler};
use roux::{client::RedditClient, util::error::RouxErrorKind};

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
            .ok_or_else(|| anyhow::anyhow!("no repo name: {}", link.path()))?;

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

impl ReadmeSlopness {
    fn reasons_for_slop(&self) -> Vec<String> {
        let mut reasons = Vec::new();

        if self.emoji_headings.ratio() > 0.75 {
            reasons.push(format!(
                "emoji-headings {}",
                Perc(self.emoji_headings.ratio())
            ));
        }

        if self.emoji_points.ratio() > 0.25 {
            reasons.push(format!(
                "emoji-list-points {}",
                Perc(self.emoji_headings.ratio())
            ));
        }

        reasons
    }
}

struct Slopness<'arena> {
    readme: ReadmeSlopness,
    /// The % of commits that are co-authored by an AI
    ai_co_authored_commits: Ratio,
    // The emitted diagnostics
    reports: Vec<BVec<'arena, Group<'arena>>>,
}

impl<'arena> std::fmt::Debug for Slopness<'arena> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Slopness")
            .field("readme", &self.readme)
            .field("ai_co_authored_commits", &self.ai_co_authored_commits)
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

trait GitCommit {
    fn sha(&self) -> &str;
    fn message(&self) -> &str;
}

impl GitCommit for RepoCommit {
    fn sha(&self) -> &str {
        &self.sha
    }
    fn message(&self) -> &str {
        &self.commit.message
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

    guess_files_slop::<C>(arena, &mut reports, &repo).await?;

    let mut ai_co_authored_commits = Ratio::default();
    let mut ai_co_author_snippets = Vec::new();

    repo.for_each_commit(client, |commit| {
        ai_co_authored_commits.total += 1;
        // TODO: add report

        if let Some(idx) = commit.message().find("Co-Authored-By: Claude") {
            let span = idx..(idx + "Co-Authored-By: Claude".len());
            ai_co_authored_commits.num += 1;

            let text = &*arena.alloc_str(commit.message());
            let sha = &*arena.alloc_str(commit.sha());

            ai_co_author_snippets.push(
                Snippet::source(text)
                    .annotation(AnnotationKind::Primary.span(span))
                    .path(sha),
            )
        }

        if ai_co_authored_commits.total >= 1000 {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    })
    .await?;

    if ai_co_authored_commits.ratio() > 0.5 {
        reports.push(bumpalo::vec![in arena; Level::ERROR
            .primary_title(format!(
                "{} commits have agentic co-authors",
                Perc(ai_co_authored_commits.ratio())
            ))
            .elements(ai_co_author_snippets.into_iter().take(10))]);
    } else if ai_co_authored_commits.ratio() > 0.25 {
        reports.push(bumpalo::vec![in arena; Level::WARNING
            .primary_title(format!(
                "{} commits have agentic co-authors",
                Perc(ai_co_authored_commits.ratio())
            ))
            .elements(ai_co_author_snippets.into_iter().take(10))]);
    }

    Ok(Slopness {
        readme,
        ai_co_authored_commits,
        reports,
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

            for phrase in ["CLAUDE.md", "GEMINI.md", "AGENTS.md", ".claude"] {
                if line.starts_with(phrase) {
                    annotations.push(
                        Snippet::source(gitignore).annotation(
                            AnnotationKind::Context
                                .span(line_span.clone())
                                .label("ignores agentic file or directory"),
                        ),
                    );
                }
            }

            offset += line.len();
        }

        if annotations.len() > 0 {
            let mut report = BVec::new_in(arena);

            report.push(
                Level::ERROR
                    .primary_title(".gitignore contains possible AI-related entries")
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

    #[derive(Debug, Clone)]
    enum WalkParent {
        List { span: Range<usize> },
        Heading { span: Range<usize> },
    }

    struct WalkCtx {
        parent: Option<WalkParent>,
        node: markdown::mdast::Node,
    }

    let mut queue = VecDeque::new();
    queue.push_back(WalkCtx {
        node: ast,
        parent: None,
    });

    while let Some(ctx) = queue.pop_front() {
        macro_rules! push_children {
            ($item:ident, !) => {
                for node in $item.children {
                    queue.push_back(WalkCtx { parent: None, node })
                }
            };
            ($item:ident, $parent:expr) => {
                for node in $item.children {
                    queue.push_back(WalkCtx {
                        parent: Some($parent.clone()),
                        node,
                    })
                }
            };
            ($item:ident) => {
                for node in $item.children {
                    queue.push_back(WalkCtx {
                        parent: ctx.parent.clone(),
                        node,
                    })
                }
            };
        }

        match ctx.node {
            Node::Text(text) => {
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
            | Node::Code(_)
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
    if chr.is_ascii_punctuation() {
        return false;
    }

    if chr.is_whitespace() {
        return false;
    }

    if chr.is_alphanumeric() {
        return false;
    }

    if ['“', '”'].contains(&chr) {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use annotate_snippets::Renderer;
    use bumpalo::Bump;
    use mlapibot_analysis::Url;

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
        //
        // ???:
        // https://github.com/landaire/stoptrackingme
        let octo = octocrab::instance();
        let url = RepoLink::parse("https://github.com/samvallad33/vestige");
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

        println!("{slopness:?}");
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
}
