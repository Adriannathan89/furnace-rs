//! Verifies conditional routes keep metadata and registration in lockstep.

use furnace_rs::common::RouteCatalog;
use furnace_rs::common::axum::body::Body;
use furnace_rs::common::axum::http::{Request, StatusCode};
use furnace_rs::common::build_router;
use furnace_rs::core::Furnace;
use tower::ServiceExt;

#[allow(dead_code)]
#[furnace_rs::routes]
trait ConditionalRoutes {
    #[cfg(feature = "conditional-route")]
    #[cfg_attr(docsrs, doc(cfg(feature = "conditional-route")))]
    #[furnace_rs::get("/conditional")]
    async fn conditional(&self) -> &'static str;
}

#[cfg(feature = "conditional-route")]
#[furnace_rs::controller(routes = [ConditionalRoutes])]
struct Controller;

#[cfg(feature = "conditional-route")]
impl ConditionalRoutes for Controller {
    #[cfg(feature = "conditional-route")]
    #[cfg_attr(docsrs, doc(cfg(feature = "conditional-route")))]
    async fn conditional(&self) -> &'static str {
        "conditional"
    }
}

#[tokio::main]
async fn main() -> furnace_rs::core::Result<()> {
    let expected = if cfg!(feature = "conditional-route") {
        (1, StatusCode::OK)
    } else {
        (0, StatusCode::NOT_FOUND)
    };
    let route_count = RouteCatalog::controllers()
        .into_iter()
        .flat_map(|controller| controller.contracts().iter())
        .flat_map(|contract| contract.routes().iter())
        .count();
    assert_eq!(route_count, expected.0);

    let application = Furnace::builder().build().await?;
    let router = build_router(&application)?;
    let response = router
        .oneshot(
            Request::builder()
                .uri("/conditional")
                .body(Body::empty())
                .expect("conditional request should build"),
        )
        .await
        .expect("router should accept the conditional request");
    assert_eq!(response.status(), expected.1);

    Ok(())
}
