//! Shared private inspection protocol fixture behavior.

use std::{
    env, fs,
    io::{self, Write},
    path::Path,
    thread,
    time::Duration,
};

use furnace_rs_common::__private::{
    INSPECTION_ACK_ENV, INSPECTION_KIND_ENV, INSPECTION_PROTOCOL_VERSION, INSPECTION_RESPONSE_ENV,
    INSPECTION_TOKEN_ENV, INSPECTION_VERSION_ENV, InspectionKind,
};

/// Runs one platform-neutral private protocol fixture behavior.
pub fn run(mode: &str) {
    if mode == "early_exit" {
        return;
    }
    if mode == "delayed_success" {
        thread::sleep(Duration::from_millis(250));
    }

    let token = env::var(INSPECTION_TOKEN_ENV).expect("token should be set");
    let ack_path = env::var(INSPECTION_ACK_ENV).expect("ack path should be set");
    let response_path = env::var(INSPECTION_RESPONSE_ENV).expect("response path should be set");
    let kind = inspection_kind(&env::var(INSPECTION_KIND_ENV).expect("kind should be set"));
    assert_eq!(
        env::var(INSPECTION_VERSION_ENV).expect("version should be set"),
        INSPECTION_PROTOCOL_VERSION.to_string()
    );

    let ack_token = if mode == "wrong_token" {
        "incorrect-token"
    } else {
        &token
    };
    publish_output(
        ack_path,
        format!(
            r#"{{"protocol_version":{},"token":"{}"}}"#,
            INSPECTION_PROTOCOL_VERSION, ack_token
        ),
    )
    .expect("ack should be written");

    if mode == "delayed_success" {
        thread::sleep(Duration::from_millis(250));
    }

    if mode == "timeout" {
        thread::sleep(Duration::from_secs(30));
        return;
    }

    if mode == "malformed" {
        publish_output(response_path, "this is not JSON").expect("response should be written");
        return;
    }

    let response_token = if mode == "wrong_token" {
        "incorrect-token"
    } else {
        &token
    };
    let version = if mode == "wrong_version" {
        INSPECTION_PROTOCOL_VERSION + 1
    } else {
        INSPECTION_PROTOCOL_VERSION
    };
    let report = format!(
        r#"{{"kind":"{}","graph":{{"root_cauldron":null,"cauldrons":[],"imports":[],"providers":[],"dependencies":[],"construction_order":null,"auto_configurations":[]}},"routes":[],"checks":[],"diagnostics":[],"failed":false}}"#,
        inspection_kind_name(kind)
    );
    publish_output(
        response_path,
        format!(r#"{{"protocol_version":{version},"token":"{response_token}","report":{report}}}"#),
    )
    .expect("response should be written");
}

fn publish_output(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
    publish_with_writer(path.as_ref(), |file| file.write_all(contents.as_ref()))
}

fn publish_with_writer(
    path: &Path,
    writer: impl FnOnce(&mut fs::File) -> io::Result<()>,
) -> io::Result<()> {
    let temporary = path.with_extension("tmp");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| {
        writer(&mut file)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    let _ = fs::remove_file(&temporary);
    result
}

const fn inspection_kind_name(kind: InspectionKind) -> &'static str {
    match kind {
        InspectionKind::Routes => "routes",
        InspectionKind::Graph => "graph",
        InspectionKind::Doctor => "doctor",
    }
}

fn inspection_kind(value: &str) -> InspectionKind {
    match value {
        "routes" => InspectionKind::Routes,
        "graph" => InspectionKind::Graph,
        "doctor" => InspectionKind::Doctor,
        _ => panic!("unexpected inspection kind"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::mpsc,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn output_is_hidden_until_the_writer_finishes() {
        let directory = env::temp_dir().join(format!(
            "furnace-protocol-publication-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let output = directory.join("response.json");
        let child_output = output.clone();
        let (started_tx, started_rx) = mpsc::channel();
        let (finish_tx, finish_rx) = mpsc::channel();
        let publisher = thread::spawn(move || {
            publish_with_writer(&child_output, |file| {
                file.write_all(b"{\"complete\":")?;
                started_tx.send(()).unwrap();
                finish_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                file.write_all(b"true}")
            })
        });
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let visible_during_write = output.exists();
        finish_tx.send(()).unwrap();
        publisher.join().unwrap().unwrap();
        let contents = fs::read(&output).unwrap();
        fs::remove_dir_all(directory).unwrap();
        assert!(
            !visible_during_write,
            "partial inspection output was visible to the supervisor"
        );
        assert_eq!(contents, b"{\"complete\":true}");
    }
}
