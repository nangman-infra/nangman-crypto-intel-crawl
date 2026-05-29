mod report;
mod rules;
mod state;

pub(crate) use report::build_source_coverage_report;

#[cfg(test)]
mod tests;
