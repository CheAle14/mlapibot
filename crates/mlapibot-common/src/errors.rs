use std::{error::Error, fmt::Write, io::stderr};

pub fn write_error_chain(mut out: impl Write, err: &impl Error) -> std::fmt::Result {
    writeln!(out, "{err}")?;

    if let Some(source) = err.source() {
        writeln!(out, "\nCaused by:")?;

        for (i, e) in std::iter::successors(Some(source), |&e| e.source()).enumerate() {
            writeln!(out, "  {i}: {e}")?;
        }
    }

    Ok(())
}

struct FmtToIo<I>(I);

impl<I> std::fmt::Write for FmtToIo<I>
where
    I: std::io::Write,
{
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        std::io::Write::write_all(&mut self.0, s.as_bytes()).map_err(|_| std::fmt::Error)
    }
}

pub fn print_error_chain(err: &impl Error) {
    let _ = write_error_chain(FmtToIo(stderr()), err);
}

pub fn to_string_error_chain(err: &impl Error) -> String {
    let mut str = String::with_capacity(128);
    let _ = write_error_chain(&mut str, err);
    str
}
