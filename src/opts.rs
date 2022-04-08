use std::ffi::OsString;

use bpaf::*;
use tracing::Level;

#[derive(Debug, Clone)]
pub enum Command {
    Explain(Explain),
    Hack(Hack),
    Restore(Option<OsString>),
    Duplicates,
    Verify,
    WorkspaceTree,
    PackageTree(String, Option<String>, Option<String>),
    ShowPackage(String, Option<String>, Option<Focus>),
    Mergedriver(OsString, OsString, OsString, OsString),
}

#[derive(Debug, Clone)]
pub enum Focus {
    Manifest,
    Readme,
    Documentation,
}

#[derive(Debug, Clone)]
pub struct Explain {
    pub krate: String,
    pub feature: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Hack {
    pub dry: bool,
    pub lock: bool,
}

fn is_version(v: &str) -> bool {
    v == "*" || semver::Version::parse(v).is_ok()
}
fn explain() -> Parser<Explain> {
    let krate = positional("CRATE");
    let feature = positional_if("FEATURE", |v| !is_version(v));
    let version = positional_if("VERSION", is_version);
    construct!(Explain {
        krate,
        feature,
        version,
    })
}

fn merge_driver_cmd() -> Parser<Command> {
    let msg = "Restore files and merge with the default merge driver";
    use Command::Mergedriver;

    let base = positional_os("BASE");
    let local = positional_os("LOCAL");
    let remote = positional_os("REMOTE");
    let result = positional_os("RESULT");
    let info = Info::default()
        .descr(msg)
        .for_parser(construct!(Mergedriver(base, local, remote, result)));
    command("merge", Some(msg), info)
}

fn explain_cmd() -> Parser<Command> {
    let msg = "Explain why a certain crate or a feature is included in the workspace";
    let info = Info::default()
        .descr(msg)
        .footer(include_str!("../doc/explain.md"))
        .for_parser(explain());

    command("explain", Some(msg), info).map(Command::Explain)
}

fn show_cmd() -> Parser<Command> {
    let msg = "Show information about a package";
    let package = positional("PACKAGE");
    let version = positional("VERSION")
        .guard(
            |s| semver::Version::parse(s).is_ok(),
            "A valid version required",
        )
        .optional();
    let show_manifest = short('m')
        .long("manifest")
        .help("Show manifest")
        .req_flag(Focus::Manifest);
    let show_readme = short('r')
        .long("readme")
        .help("Show readme")
        .req_flag(Focus::Readme);
    let show_doc = short('d')
        .long("doc")
        .help("Open documentation URL")
        .req_flag(Focus::Documentation);
    let focus = show_manifest
        .or_else(show_readme)
        .or_else(show_doc)
        .optional();
    use Command::ShowPackage;
    let info = Info::default()
        .descr(msg)
        .for_parser(construct!(ShowPackage(package, version, focus)));
    command("show", Some(msg), info)
}

fn hack_cmd() -> Parser<Command> {
    let msg = "Unify crate dependencies across individual crates in the workspace";
    let dry = dry_run();
    let lock = short('l')
        .long("lock")
        .help("Include dependencies checksum into stash")
        .switch();
    let info = Info::default()
        .descr(msg)
        .footer(include_str!("../doc/hack.md"))
        .for_parser(construct!(Hack { dry, lock }));
    command("hack", Some(msg), info).map(Command::Hack)
}

fn restore_cmd() -> Parser<Command> {
    let file = positional_os("FILE").optional();

    let info = Info::default()
        .descr("Remove crate dependency unification added by the 'hack' command")
        .for_parser(file.map(Command::Restore));
    command("restore", Some("Remove unification"), info)
}

fn verify_cmd() -> Parser<Command> {
    let info = Info::default()
        .descr("Check if unification is required and other invariants")
        .for_parser(Parser::pure(()));
    command(
        "check",
        Some("Check for unification and other issues"),
        info,
    )
    .map(|_| Command::Verify)
}

fn duplicates_cmd() -> Parser<Command> {
    let descr = "Lists all the duplicates in the workspace";
    let info = Info::default().descr(descr).for_parser(Parser::pure(()));
    command("dupes", Some(descr), info).map(|_| Command::Duplicates)
}

fn tree_cmd() -> Parser<Command> {
    let descr = "Display crates dependencies as a tree";

    let package = positional("CRATE").optional();
    let feature = positional_if("FEATURE", |v| !is_version(v));
    let version = positional("VERSION").optional().guard(
        |x| x.is_none() || semver::Version::parse(x.as_ref().unwrap()).is_ok(),
        "You need to specify a valid semver compatible version",
    );
    let p = construct!(package, feature, version);
    let info = Info::default()
        .descr(descr)
        .footer(include_str!("../doc/tree.md"))
        .for_parser(p);
    command("tree", Some(descr), info).map(|args| match args {
        (Some(p), feat, ver) => Command::PackageTree(p, feat, ver),
        (None, _, _) => Command::WorkspaceTree,
    })
}

fn verbosity() -> Parser<Level> {
    short('v')
        .help("increase verbosity, can be used several times")
        .req_flag(())
        .many()
        .map(|xs| match xs.len() {
            0 => Level::WARN,
            1 => Level::INFO,
            2 => Level::DEBUG,
            _ => Level::TRACE,
        })
}

fn dry_run() -> Parser<bool> {
    short('d')
        .long("dry")
        .help("report actions to be performed without actually performing them")
        .switch()
}

pub fn options() -> OptionParser<(Level, OsString, Command)> {
    Info::default().for_parser(command(
        "hackerman",
        Some("A set of commands to do strange things to the workspace"),
        options_inner(),
    ))
}

fn custom_manifest() -> Parser<OsString> {
    long("manifest-path")
        .help("Path to Cargo.toml")
        .argument_os("PATH")
        .fallback("Cargo.toml".into())
}

// For reasons (?) cargo doesn't replace the command line used so we need to put a command inside a
// command.
fn options_inner() -> OptionParser<(Level, OsString, Command)> {
    let v = verbosity();
    let cmd = explain_cmd()
        .or_else(hack_cmd())
        .or_else(restore_cmd())
        .or_else(duplicates_cmd())
        .or_else(verify_cmd())
        .or_else(tree_cmd())
        .or_else(merge_driver_cmd())
        .or_else(show_cmd());
    let custom_manifest = custom_manifest();
    let opts = construct!(v, custom_manifest, cmd);
    Info::default().for_parser(opts)
}
