pub const fn help_text() -> &'static str {
    concat!(
        "    YatSenOS shell v",
        env!("CARGO_PKG_VERSION"),
        " by GZTime",
        r#"

Usage:
    help        | show this help
    ps          | show process list
    ls          | show app list
    exec <name> | execute program
    kill <pid>  | kill process
    clear       | clear screen
    exit        | exit shell

Shortcuts:
    Ctrl + D    | exit shell
    Ctrl + C    | cancel current command
"#
    )
}
