//! Locally generated certificate material used only by integration tests.
use rcgen::{BasicConstraints, CertificateParams, IsCa, Issuer, KeyPair, KeyUsagePurpose};
pub struct Certificates {
    pub _directory: tempfile::TempDir,
    pub ca: std::path::PathBuf,
    pub certificate: std::path::PathBuf,
    pub key: std::path::PathBuf,
}
impl Certificates {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        let ca_key = KeyPair::generate().unwrap();
        let ca_cert = ca_params.self_signed(&ca_key).unwrap();
        let issuer = Issuer::from_params(&ca_params, &ca_key);
        let server_params = CertificateParams::new(vec!["localhost".to_owned()]).unwrap();
        let key_pair = KeyPair::generate().unwrap();
        let server_cert = server_params.signed_by(&key_pair, &issuer).unwrap();
        let ca = directory.path().join("ca.pem");
        let certificate = directory.path().join("server.pem");
        let key = directory.path().join("server-key.pem");
        std::fs::write(&ca, ca_cert.pem()).unwrap();
        std::fs::write(&certificate, server_cert.pem()).unwrap();
        std::fs::write(&key, key_pair.serialize_pem()).unwrap();
        Self {
            _directory: directory,
            ca,
            certificate,
            key,
        }
    }
}
