//! V108-03 Command lines: split the way Windows does (CommandLineToArgvW) and the profile
//! flags a browser was started with, so a sign-in lands in that same profile.

/// The browser families whose profile flags differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Firefox,
    Chromium,
}

/// Firefox and its forks (one process per profile); everything else is Chromium-based.
const FIREFOX: [&str; 8] = [
    "firefox",
    "librewolf",
    "waterfox",
    "zen",
    "mullvadbrowser",
    "floorp",
    "palemoon",
    "torbrowser",
];

/// `name`: lowercase exe name without `.exe`.
pub fn family(name: &str) -> Family {
    if FIREFOX.contains(&name) {
        Family::Firefox
    } else {
        Family::Chromium
    }
}

/// The flags to start `family` with so the URL reaches the running instance's profile.
/// `argv[0]` is the exe.
///
/// Firefox runs one process per profile, so `-P <name>` / `-profile <path>` (and
/// `-no-remote`) name the window's profile; none means the default one. Chromium runs all
/// profiles of a user-data dir in one process and opens a URL in the profile used last, i.e.
/// the window that was active; its start-up `--profile-directory` may name another profile,
/// so only `--user-data-dir` (which instance) is kept.
pub fn profile_args(family: Family, argv: &[String]) -> Vec<String> {
    let rest = argv.get(1..).unwrap_or_default();
    match family {
        Family::Firefox => firefox(rest),
        Family::Chromium => chromium(rest)
            .user_data_dir
            .map(|d| vec![format!("--user-data-dir={d}")])
            .unwrap_or_default(),
    }
}

fn firefox(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let is_flag = args[i].starts_with(['-', '/']);
        let flag = args[i].trim_start_matches(['-', '/']).to_ascii_lowercase();
        let next = args.get(i + 1).filter(|v| !v.is_empty() && !v.starts_with('-'));
        match (is_flag, flag.as_str(), next) {
            (true, "p", Some(v)) => {
                out.extend(["-P".to_owned(), v.clone()]);
                i += 1;
            }
            (true, "profile", Some(v)) => {
                out.extend(["-profile".to_owned(), v.clone()]);
                i += 1;
            }
            (true, "no-remote", _) => out.push("-no-remote".to_owned()),
            _ => {}
        }
        i += 1;
    }
    out
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Chromium {
    pub user_data_dir: Option<String>,
    pub profile_directory: Option<String>,
}

/// `--user-data-dir=…` and `--profile-directory=…` (the last one wins, as in Chromium).
pub fn chromium(args: &[String]) -> Chromium {
    let mut out = Chromium::default();
    for a in args {
        let Some((k, v)) = a.strip_prefix("--").and_then(|a| a.split_once('=')) else {
            continue;
        };
        let v = v.trim_matches('"');
        if v.is_empty() {
            continue;
        }
        match k.to_ascii_lowercase().as_str() {
            "user-data-dir" => out.user_data_dir = Some(v.to_owned()),
            "profile-directory" => out.profile_directory = Some(v.to_owned()),
            _ => {}
        }
    }
    out
}

/// Splits a command line like CommandLineToArgvW: the exe ends at the closing quote or the
/// first blank; then blanks separate, quotes group, `2n` backslashes before a quote are `n`
/// backslashes, `2n+1` are `n` and a literal quote, and `""` inside quotes is a quote.
pub fn split(line: &str) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    // The exe: no escapes.
    let mut exe = String::new();
    if chars.first() == Some(&'"') {
        i = 1;
        while i < chars.len() && chars[i] != '"' {
            exe.push(chars[i]);
            i += 1;
        }
        i += 1;
    } else {
        while i < chars.len() && !chars[i].is_whitespace() {
            exe.push(chars[i]);
            i += 1;
        }
    }
    out.push(exe);
    loop {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() {
            return out;
        }
        let mut arg = String::new();
        let mut quoted = false;
        while i < chars.len() && (quoted || !chars[i].is_whitespace()) {
            match chars[i] {
                '\\' => {
                    let start = i;
                    while i < chars.len() && chars[i] == '\\' {
                        i += 1;
                    }
                    let n = i - start;
                    if chars.get(i) == Some(&'"') {
                        arg.extend(std::iter::repeat_n('\\', n / 2));
                        if n % 2 == 1 {
                            arg.push('"');
                            i += 1;
                        }
                    } else {
                        arg.extend(std::iter::repeat_n('\\', n));
                    }
                }
                '"' if quoted && chars.get(i + 1) == Some(&'"') => {
                    arg.push('"');
                    i += 2;
                }
                '"' => {
                    quoted = !quoted;
                    i += 1;
                }
                c => {
                    arg.push(c);
                    i += 1;
                }
            }
        }
        out.push(arg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| (*x).to_owned()).collect()
    }

    #[test]
    fn splits_like_windows() {
        assert_eq!(
            split(r#""C:\Program Files\Mozilla Firefox\firefox.exe" -P "Work Twitch" -no-remote"#),
            s(&[
                r"C:\Program Files\Mozilla Firefox\firefox.exe",
                "-P",
                "Work Twitch",
                "-no-remote"
            ])
        );
        assert_eq!(
            split(r"C:\b\brave.exe  --flag"),
            s(&[r"C:\b\brave.exe", "--flag"])
        );
        assert_eq!(
            split(r#"x.exe "--user-data-dir=C:\Users\a b\Data" --profile-directory="Profile 2""#),
            s(&[
                "x.exe",
                r"--user-data-dir=C:\Users\a b\Data",
                "--profile-directory=Profile 2"
            ])
        );
        // Backslashes: literal unless before a quote.
        assert_eq!(
            split(r#"x a\\b "c\\" d\"e "f""g""#),
            s(&["x", r"a\\b", r"c\", "d\"e", "f\"g"])
        );
        assert_eq!(split(r#"x -profile "C:\p\""#), s(&["x", "-profile", r#"C:\p""#]));
        assert_eq!(split(""), s(&[""]));
    }

    #[test]
    fn firefox_profiles() {
        let f = |line: &str| profile_args(Family::Firefox, &split(line));
        assert!(
            f(r#""C:\Program Files\Mozilla Firefox\firefox.exe""#).is_empty(),
            "default profile"
        );
        assert!(f("firefox.exe -osint -url https://a.example/").is_empty());
        assert_eq!(
            f(r#"firefox.exe -P "Second Twitch""#),
            s(&["-P", "Second Twitch"])
        );
        assert_eq!(
            f("firefox.exe -p alt -no-remote"),
            s(&["-P", "alt", "-no-remote"])
        );
        assert_eq!(
            f(r#"firefox.exe --profile "C:\Users\a b\AppData\Roaming\Mozilla\Firefox\Profiles\x.alt""#),
            s(&[
                "-profile",
                r"C:\Users\a b\AppData\Roaming\Mozilla\Firefox\Profiles\x.alt"
            ])
        );
        assert!(
            f("firefox.exe -P").is_empty(),
            "-P alone opens the profile manager"
        );
        assert!(f("firefox.exe -P -private").is_empty());
        assert!(f("firefox.exe -private -purgecaches").is_empty());
    }

    #[test]
    fn chromium_profiles() {
        let line = r#""C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe" --profile-directory="Profile 3" "--user-data-dir=D:\Brave Data""#;
        assert_eq!(
            chromium(&split(line)[1..]),
            Chromium {
                user_data_dir: Some(r"D:\Brave Data".into()),
                profile_directory: Some("Profile 3".into())
            }
        );
        assert_eq!(
            profile_args(Family::Chromium, &split(line)),
            s(&[r"--user-data-dir=D:\Brave Data"])
        );
        assert!(profile_args(Family::Chromium, &split("chrome.exe --profile-directory=Default")).is_empty());
        assert!(profile_args(Family::Chromium, &split("chrome.exe")).is_empty());
        assert_eq!(chromium(&s(&["--user-data-dir="])), Chromium::default());
    }

    #[test]
    fn families() {
        assert_eq!(family("firefox"), Family::Firefox);
        assert_eq!(family("mullvadbrowser"), Family::Firefox);
        assert_eq!(family("brave"), Family::Chromium);
        assert_eq!(family("thorium"), Family::Chromium);
    }
}
