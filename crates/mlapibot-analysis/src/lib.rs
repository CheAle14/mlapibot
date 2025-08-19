pub mod analzyer;
pub mod error;
pub mod matcher;

mod context;
mod url;
mod util;

pub use util::{extract_all_links, parse_url};

pub use context::*;
pub use url::*;

use analzyer::Analyzer;
use mlapibot_common::Detection;

pub fn load_scams() -> serde_json::Result<Vec<Analyzer>> {
    static FILE: &str = include_str!("../../../data/scams.json");

    #[derive(serde::Deserialize)]
    struct SaveFile {
        pub scams: Vec<Analyzer>,
    }

    let scams: SaveFile = serde_json::from_str(FILE)?;
    Ok(scams.scams)
}

pub fn get_best_analysis<'yzer>(
    ctx: &Context,
    analyzer: &'yzer [Analyzer],
) -> crate::error::Result<Option<(Detection, &'yzer Analyzer)>> {
    let mut best: Option<(f32, Detection, &Analyzer)> = None;
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
