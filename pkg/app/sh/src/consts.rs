pub const fn help_text() -> &'static str {
    concat!(
        "    YatSenOS shell v",
        env!("CARGO_PKG_VERSION"),
        " by GZTime",
        r#"

Usage:
    help        | show this help
    ps          | show process list
    ls          | list directory
    cd <path>   | change directory
    cat <file>  | show file content
    exec <file> | execute file
    nohup <file>| execute file in background
    kill <pid>  | kill process
    clear       | clear screen
    exit        | exit shell

Shortcuts:
    Ctrl + D    | exit shell
    Ctrl + C    | cancel current command

"#
    )
}
