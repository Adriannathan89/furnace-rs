//! String-array paths resolve from their winning source, just like scalar paths.
use furnace_rs_core::{ConfigBuilder, MapSource, TomlSource};

#[test]
fn array_paths_follow_winning_source_base() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("settings.toml");
    let absolute = root.path().join("absolute.pem");
    std::fs::write(
        &source,
        format!("roots = ['ca.pem', '{}']\n", absolute.display()),
    )
    .unwrap();
    let config = ConfigBuilder::new()
        .source(TomlSource::file(source))
        .build()
        .unwrap();
    assert_eq!(
        config.resolve_string_array_path("roots", 0).unwrap(),
        root.path().join("ca.pem")
    );
    assert_eq!(
        config.resolve_string_array_path("roots", 1).unwrap(),
        absolute
    );
    assert!(config.resolve_string_array_path("roots", 2).is_none());
    let override_config = ConfigBuilder::new()
        .source(
            MapSource::new("test", [] as [(&str, &str); 0])
                .with_string_array("roots", ["override.pem"]),
        )
        .build()
        .unwrap();
    assert_eq!(
        override_config
            .resolve_string_array_path("roots", 0)
            .unwrap(),
        std::env::current_dir().unwrap().join("override.pem")
    );
}
