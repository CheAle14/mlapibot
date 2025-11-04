#![feature(mpmc_channel)]
use std::{
    ops::Range,
    path::{Path, PathBuf},
    sync::atomic::AtomicU32,
};

use anyhow::Context;
use mlapibot_analysis::analzyer::Analyzer;

fn collect_scam_tests(tests_dir: &Path) -> anyhow::Result<Vec<TestFile>> {
    let mut tests = Vec::new();

    for dir in tests_dir.read_dir().context("list tests dir")? {
        let dir = dir.context("read dir entry")?;
        if dir.file_type().context("get dir entry type")?.is_file() {
            continue;
        }

        let path = dir.path();
        let folder_name = path
            .file_name()
            .expect("has last segment")
            .to_str()
            .expect("is utf8");
        let expected = if folder_name == "none" {
            None
        } else {
            Some(folder_name.to_string())
        };

        println!("- {path:?}");
        for file in path.read_dir().context("list inner test dir")? {
            let file = file.context("read file entry")?;

            if !file.file_type().context("get file entry type")?.is_file() {
                continue;
            }

            let path = file.path();

            if path.extension().expect("has extension") == "disabled" {
                continue;
            }

            println!("  - {file:?}");

            tests.push(TestFile {
                path,
                expected: expected.clone(),
            });
        }
    }

    Ok(tests)
}

struct Results {
    trp: AtomicU32,
    trn: AtomicU32,
    flp: AtomicU32,
    fln: AtomicU32,
}

macro_rules! foreach {
    ($($fn:ident => $field:ident),* $(,)?) => {
        $(
            fn $fn(&self) {
                self.$field.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        )*
    };
}

impl Results {
    pub fn new() -> Self {
        Self {
            trp: AtomicU32::new(0),
            trn: AtomicU32::new(0),
            flp: AtomicU32::new(0),
            fln: AtomicU32::new(0),
        }
    }

    pub fn finish(self) -> (u32, u32, u32, u32) {
        (
            self.trp.load(std::sync::atomic::Ordering::Relaxed),
            self.trn.load(std::sync::atomic::Ordering::Relaxed),
            self.flp.load(std::sync::atomic::Ordering::Relaxed),
            self.fln.load(std::sync::atomic::Ordering::Relaxed),
        )
    }

    foreach!(
        true_pos => trp,
        true_neg => trn,
        false_pos => flp,
        false_neg => fln,
    );
}

#[test]
pub fn test_registered_images() -> anyhow::Result<()> {
    let tests_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").context("read manifest env")?);
    let analyzers = mlapibot_analysis::load_scams().context("load scams")?;

    let tests = collect_scam_tests(&tests_dir).context("collect tests")?;
    println!("Collected {} test images", tests.len());

    let out = Results::new();

    let tests_ref = tests.as_slice();
    let analyzers_ref = analyzers.as_slice();
    let out_ref = &out;

    let mut thread_idx = 0;
    std::thread::scope(|scope| {
        let mut iter = split(tests_ref, num_cpus::get());

        while let Some(range) = iter.next() {
            scope.spawn(move || {
                for item in range {
                    if let Err(err) = item.run(thread_idx, analyzers_ref, out_ref) {
                        eprintln!("ERR : {err:?}");
                    }
                }
            });
            thread_idx += 1;
        }
    });

    let (trp, trn, flp, fln) = out.finish();

    let correct = trp + trn;
    let incorrect = flp + fln;

    println!("tp={trp} tn={trn} fp={flp} fn={fln}");
    let total = correct + incorrect;
    println!("{correct} out of {total}");

    let false_pos_ratio = flp as f32 / (flp as f32 + trn as f32);
    println!("FPR: {false_pos_ratio}");

    if fln > 0 || false_pos_ratio > 0.05 {
        anyhow::bail!("failed {fln} tests or {false_pos_ratio} too high")
    } else {
        Ok(())
    }
}

// From https://users.rust-lang.org/t/how-to-split-a-slice-into-n-chunks/40008/2
pub fn split<T>(slice: &[T], n: usize) -> impl Iterator<Item = &[T]> {
    let len = slice.len() / n;
    let rem = slice.len() % n;
    Split { slice, len, rem }
}

struct Split<'a, T> {
    slice: &'a [T],
    len: usize,
    rem: usize,
}

impl<'a, T> Iterator for Split<'a, T> {
    type Item = &'a [T];

    fn next(&mut self) -> Option<Self::Item> {
        if self.slice.is_empty() {
            return None;
        }
        let mut len = self.len;
        if self.rem > 0 {
            len += 1;
            self.rem -= 1;
        }
        let (chunk, rest) = self.slice.split_at(len);
        self.slice = rest;
        Some(chunk)
    }
}

#[derive(Debug)]
struct TestFile {
    path: PathBuf,
    expected: Option<String>,
}

impl TestFile {
    fn run(&self, idx: usize, analyzers: &[Analyzer], out: &Results) -> anyhow::Result<()> {
        let context = mlapibot_analysis::Context::new_path(&self.path)
            .with_context(|| format!("{idx} {self:?}"))?;

        let result = mlapibot_analysis::get_best_analysis(&context, &analyzers)
            .with_context(|| format!("{idx} {self:?}"))?;

        match (result, &self.expected) {
            (None, None) => {
                println!("{idx} OK N: {:?}", self.path);
                out.true_neg();
            }
            (Some((_, actual)), Some(exp)) if &actual.name == exp => {
                println!("{idx} OK S: {:?}", self.path);
                out.true_pos();
            }
            (Some((_, actual)), Some(exp)) => {
                println!(
                    "{idx} FAIL: incorrect {}, wanted {} for {:?}",
                    actual.name, exp, self.path
                );
                out.false_pos();
            }
            (Some((_, actual)), None) => {
                println!("{idx} FAIL: unexpected {} for {:?}", actual.name, self.path);
                out.false_pos();
            }
            (None, Some(exp)) => {
                println!("{idx} FAIL: required {} for {:?}", exp, self.path);
                out.false_neg();
            }
        }

        Ok(())
    }
}
