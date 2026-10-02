//! Verifies controller expansion through a renamed common dependency.

#[web::controller]
struct Controller;

impl ::web::Sealable for Controller {
    fn seals() -> ::web::SealRegistration<Self> {
        ::web::SealRegistration::new()
    }
}

#[web::controller]
impl Controller {
    #[web::get("/")]
    async fn index(&self) {}
}

async fn build_application() -> web::core::Result<()> {
    let application = web::core::Furnace::builder().build().await?;
    let _router = web::build_router(&application)?;
    Ok(())
}

#[derive(web::PassportPrincipal)]
struct Principal {
    #[roles]
    roles: Vec<String>,
}

fn passport_principal() {
    let principal = Principal {
        roles: vec!["member".into()],
    };
    assert!(web::PassportPrincipal::has_role(&principal, "member"));
}

fn main() {
    use web::Input;
    #[derive(web::Input)]
    struct RequestInput {
        #[validate(email)]
        email: String,
    }
    assert!(
        RequestInput {
            email: "user@example.com".into()
        }
        .validate()
        .is_ok()
    );
    let _ = passport_principal;
    let _ = build_application;
}
