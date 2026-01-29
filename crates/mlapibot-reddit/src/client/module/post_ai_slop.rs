use std::{
    collections::{HashSet, VecDeque},
    ops::ControlFlow,
};

use futures_util::TryStreamExt;
use markdown::mdast::Node;
use mlapibot_analysis::{Url, extract_all_links};
use octocrab::{Octocrab, models::repos::RepoCommit, repos::RepoHandler};

use crate::client::module::impl_mask_subreddits;

pub struct PostAiSlop {}

impl super::Module for PostAiSlop {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {}
    }

    fn name(&self) -> &'static str {
        "post_ai_slop"
    }

    fn wants(&self) -> super::ModuleWants {
        super::ModuleWants::POSTS
    }

    impl_mask_subreddits!(ai_slop => posts);

    fn run_post<'client>(
        &mut self,
        client: &mut crate::client::ModuleRedditClient<'client>,
        subreddit: &mut crate::client::Subreddit,
        config: Option<&crate::config::SubredditConfig>,
        post: &crate::Submission,
        has_seen: bool,
    ) -> anyhow::Result<super::PostAction> {
        let Some(github) = client.github else {
            return Ok(super::PostAction::Ignore);
        };

        let mut links = Vec::new();
        if let Some(Ok(url)) = post.url().as_ref().map(|v| Url::parse(&v)) {
            links.push(url);
        }

        for link in extract_all_links(post.selftext(), None) {
            links.push(link);
        }

        for link in links {
            if !link.domain().ends_with("github.com") {
                continue;
            }

            match determine_ai_slop(&*github, &link) {
                Ok(slop) => {
                    println!("{slop:?}");
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

#[derive(Debug, Default, PartialEq)]
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

#[derive(Debug)]
struct Slopness {
    readme: ReadmeSlopness,
    /// The % of commits that are co-authored by an AI
    ai_co_authored_commits: Ratio,
}

trait GitClient: Sized {
    type Repository<'l>: GitRepository<Self>
    where
        Self: 'l;

    fn open_repo<'l>(&'l self, owner: &str, name: &str) -> anyhow::Result<Self::Repository<'l>>;
}

impl GitClient for Octocrab {
    type Repository<'l> = octocrab::repos::RepoHandler<'l>;

    fn open_repo<'l>(&'l self, owner: &str, name: &str) -> anyhow::Result<Self::Repository<'l>> {
        let repo = self.repos(owner, name);
        Ok(repo)
    }
}

trait GitRepository<C: GitClient> {
    type Commit: GitCommit;

    fn fetch_readme(&self) -> anyhow::Result<String>;

    fn for_each_commit<F>(&self, client: &C, callback: F) -> anyhow::Result<()>
    where
        F: FnMut(Self::Commit) -> ControlFlow<()>;
}

impl<'l> GitRepository<Octocrab> for RepoHandler<'l> {
    type Commit = octocrab::models::repos::RepoCommit;

    fn fetch_readme(&self) -> anyhow::Result<String> {
        let rt = tokio::runtime::Handle::current();
        let content = rt.block_on(async { self.get_readme().send().await })?;

        Ok(content.decoded_content().unwrap_or_default())
    }

    fn for_each_commit<F>(&self, client: &Octocrab, mut callback: F) -> anyhow::Result<()>
    where
        F: FnMut(Self::Commit) -> ControlFlow<()>,
    {
        println!("for-each-commit");
        let rt = tokio::runtime::Handle::current();

        rt.block_on(async {
            let s = self.list_commits().send().await?.into_stream(client);

            tokio::pin!(s);

            while let Some(cmm) = s.try_next().await? {
                match callback(cmm) {
                    ControlFlow::Continue(()) => continue,
                    ControlFlow::Break(()) => break,
                }
            }

            Ok(())
        })
    }
}

trait GitCommit {
    fn message(&self) -> &str;
}

impl GitCommit for RepoCommit {
    fn message(&self) -> &str {
        &self.commit.message
    }
}

fn determine_ai_slop<C: GitClient>(client: &C, link: &Url) -> anyhow::Result<Slopness> {
    if !link.domain().ends_with("github.com") {
        anyhow::bail!("only github.com supported, was: {}", link.domain());
    }

    let mut iter = link.path().split('/');
    let repo_owner = iter
        .next()
        .ok_or_else(|| anyhow::anyhow!("no repo owner: {}", link.path()))?;
    let repo_name = iter
        .next()
        .ok_or_else(|| anyhow::anyhow!("no repo name: {}", link.path()))?;

    let repo = client.open_repo(repo_owner, repo_name)?;

    let readme = repo.fetch_readme()?;

    let readme = guess_readme_slop(&readme)?;

    let mut ai_co_authored_commits = Ratio::default();

    repo.for_each_commit(client, |commit| {
        ai_co_authored_commits.total += 1;
        if commit.message().contains("Claude") {
            ai_co_authored_commits.num += 1;
        }

        if ai_co_authored_commits.total >= 1000 {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    })?;

    Ok(Slopness {
        readme,
        ai_co_authored_commits,
    })
}

fn guess_readme_slop(readme: &str) -> anyhow::Result<ReadmeSlopness> {
    let mut slopness = ReadmeSlopness::default();
    let mut next_list_id = 0;
    let mut seen_emoji_in_lists = HashSet::new();

    let ast = markdown::to_mdast(readme, &markdown::ParseOptions::gfm()).map_err(|e| {
        anyhow::anyhow!(
            "failed to parse: {} (rule_id={}, source={}, place={:?})",
            e.reason,
            e.rule_id,
            e.source,
            e.place
        )
    })?;

    #[derive(Debug, Clone, Copy)]
    enum WalkParent {
        List { id: u32 },
        Heading,
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
            ($item:ident, $parent:expr) => {
                for node in $item.children {
                    queue.push_back(WalkCtx {
                        parent: Some($parent),
                        node,
                    })
                }
            };
            ($item:ident) => {
                for node in $item.children {
                    queue.push_back(WalkCtx {
                        parent: ctx.parent,
                        node,
                    })
                }
            };
        }

        match ctx.node {
            Node::Text(text) => {
                let mut saw_emoji = false;
                for chr in text.value.chars() {
                    slopness.total_chars += 1;
                    if chr == '—' {
                        slopness.num_em_dash += 1;
                    } else if is_char_emoji(chr) {
                        slopness.num_emoji += 1;

                        if saw_emoji {
                            continue;
                        }

                        saw_emoji = true;
                        match ctx.parent {
                            Some(WalkParent::Heading) => slopness.emoji_headings.num += 1,
                            Some(WalkParent::List { id }) if seen_emoji_in_lists.insert(id) => {
                                println!("Flag emoji {chr:?} {:?}: {:?}", ctx.parent, text.value);
                                slopness.emoji_points.num += 1
                            }
                            _ => (),
                        }
                    }
                }
            }
            Node::Root(root) => push_children!(root),
            Node::Heading(heading) => {
                slopness.emoji_headings.total += 1;
                push_children!(heading, WalkParent::Heading);
            }
            Node::ListItem(v) => {
                slopness.emoji_points.total += 1;

                println!("LIST: {v:?}");

                let id = next_list_id;
                next_list_id += 1;
                push_children!(v, WalkParent::List { id })
            }

            Node::List(heading) => push_children!(heading),
            Node::Paragraph(v) => push_children!(v),
            Node::Blockquote(v) => push_children!(v),
            Node::FootnoteDefinition(v) => push_children!(v),
            Node::MdxJsxFlowElement(v) => push_children!(v),
            Node::Delete(v) => push_children!(v),
            Node::Emphasis(v) => push_children!(v),
            Node::MdxJsxTextElement(v) => push_children!(v),
            Node::Link(v) => push_children!(v),
            Node::LinkReference(v) => push_children!(v),
            Node::Strong(v) => push_children!(v),
            Node::Table(v) => push_children!(v),
            Node::TableRow(v) => push_children!(v),
            Node::TableCell(v) => push_children!(v),

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

    Ok(slopness)
}

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
    use mlapibot_analysis::Url;
    use octocrab::Octocrab;

    use crate::client::module::post_ai_slop::{Ratio, ReadmeSlopness};

    #[test]
    fn our_readme_is_not_slop() {
        static THIS_README: &str = include_str!("../../../../../README.md");

        let slop = super::guess_readme_slop(THIS_README).unwrap();

        assert_eq!(slop.num_em_dash, 0);
        assert_eq!(slop.num_emoji, 0);
        assert_eq!(slop.emoji_points.ratio(), 0.0);
        assert_eq!(slop.emoji_headings.ratio(), 0.0);
    }

    #[test]
    fn rustc_is_not_slop() {
        println!("init");

        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let octo = octocrab::instance();
                let url = Url::parse("https://github.com/rust-lang/rust").unwrap();
                println!("determine");

                let slopness = super::determine_ai_slop(octo.as_ref(), &url).unwrap();
                println!("{slopness:?}");
            });
    }

    #[test]
    fn fake_readme_is_slop() {
        /// Generated by ChatGPT, naturally.
        static SLOP_README: &str = include_str!("slop_readme.md");

        let slop = super::guess_readme_slop(SLOP_README).unwrap();

        let points = slop.emoji_points.ratio();
        let headings = slop.emoji_headings.ratio();
        let emoji = slop.num_emoji as f32 / slop.total_chars as f32;

        assert_eq!(
            slop,
            ReadmeSlopness {
                emoji_points: Ratio { num: 9, total: 33 },
                emoji_headings: Ratio { num: 12, total: 12 },
                num_emoji: 49,
                num_em_dash: 0,
                total_chars: 2601
            }
        );
    }
}
