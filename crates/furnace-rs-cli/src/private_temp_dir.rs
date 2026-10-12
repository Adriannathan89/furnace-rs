//! Temporary directories for private CLI control and inspection transport.

pub(crate) fn create() -> std::io::Result<tempfile::TempDir> {
    // Set the mode on mkdir itself: chmod after creation would leave a
    // window in which another local user could alter our control files.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
    }
    #[cfg(not(unix))]
    {
        tempfile::tempdir()
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::{
        os::unix::fs::PermissionsExt,
        process::{Command, Output},
    };

    const CHILD_ENV: &str = "FURNACE_PRIVATE_DIRECTORY_TEST_CHILD";
    const CHILD_TEST: &str = "private_temp_dir::tests::private_directory_child";
    const CHILD_COMPLETE: &str = "FURNACE_PRIVATE_DIRECTORY_CHECK_COMPLETE";

    fn child_completed(output: &Output) -> bool {
        output.status.success()
            && String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| *line == CHILD_COMPLETE)
                .count()
                == 1
    }

    #[test]
    fn an_absent_child_test_cannot_be_counted_as_a_security_check() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "private_temp_dir::tests::nonexistent_child"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "libtest's empty selection exits successfully"
        );
        assert!(
            !child_completed(&output),
            "zero child tests cannot prove directory privacy"
        );
    }

    #[test]
    fn private_directories_exclude_other_users_under_each_umask() {
        let rounds = std::env::var("FURNACE_CLI_SECURITY_ROUNDS")
            .map(|value| value.parse::<usize>().expect("rounds must be an integer"))
            .unwrap_or(1);
        assert!((1..=10_000).contains(&rounds));
        let executable = std::env::current_exe().unwrap();
        let mut failures = 0;
        for _ in 0..rounds {
            for umask in ["000", "002", "022", "077"] {
                // The umask changes only in this child, never in the parallel
                // test runner. Paths and arguments remain separate shell argv.
                let output = Command::new("sh")
                    .args([
                        "-c",
                        "umask \"$1\"; exec \"$2\" --exact \"$3\" --nocapture",
                        "furnace-private-directory-test",
                        umask,
                    ])
                    .arg(&executable)
                    .arg(CHILD_TEST)
                    .env(CHILD_ENV, "1")
                    .output()
                    .unwrap();
                if !child_completed(&output) {
                    failures += 1;
                    eprintln!(
                        "umask {umask}: {}{}",
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
            }
        }
        println!(
            "FURNACE_CLI_SECURITY_RESULT {{\"rounds\":{rounds},\"checks\":{},\"failures\":{failures},\"passed\":{}}}",
            rounds * 4,
            failures == 0,
        );
        assert_eq!(
            failures, 0,
            "private CLI directories exposed to other users"
        );
    }

    #[test]
    fn private_directory_child() {
        if std::env::var_os(CHILD_ENV).is_none() {
            return;
        }
        let directory = super::create().unwrap();
        let mode = directory.path().metadata().unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700, "control directory mode was {mode:o}");
        // The owner retains the operations needed by the real transports.
        let response = directory.path().join("response.json");
        std::fs::write(&response, b"private transport sentinel").unwrap();
        assert_eq!(
            std::fs::read(response).unwrap(),
            b"private transport sentinel"
        );
        println!("{CHILD_COMPLETE}");
    }
}
