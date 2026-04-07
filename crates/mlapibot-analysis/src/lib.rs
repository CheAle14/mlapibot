pub mod analzyer;
pub mod error;
pub mod matcher;

mod context;
mod url;
mod util;

pub use util::{download_file, extract_all_links, parse_url};

pub use context::*;
pub use url::*;

use analzyer::Analyzer;
use mlapibot_common::Detection;

pub fn get_best_analysis<'yzer, A: Analyzer>(
    ctx: &Context,
    analyzer: &'yzer [A],
) -> crate::error::Result<Option<(Detection, &'yzer A)>> {
    let mut best: Option<(f32, Detection, &A)> = None;
    for next in analyzer {
        if let Some(detection) = next.analyze(ctx)? {
            let score = detection.best_score();
            if best.is_none() || score > best.as_ref().unwrap().0 {
                best = Some((score, detection, next));
                if score >= 1.0 {
                    break;
                }
            }
        }
    }

    let best = best.map(|(_, d, a)| (d, a));
    Ok(best)
}
