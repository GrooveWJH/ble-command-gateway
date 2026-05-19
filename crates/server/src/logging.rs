use tracing_subscriber::fmt::format::FmtSpan;

pub fn init_logging() {
    let use_color = match std::env::var("YUNDRONE_LOG_COLOR") {
        Ok(value) if value.eq_ignore_ascii_case("always") => true,
        Ok(value) if value.eq_ignore_ascii_case("never") => false,
        _ => true,
    };

    tracing_subscriber::fmt()
        .with_ansi(use_color)
        .with_target(true)
        .with_level(true)
        .with_span_events(FmtSpan::NONE)
        .init();
}
