use furnace_rs::prelude::*;

use super::{
    model::{Post, PostInput},
    service::PostService,
};

#[controller]
pub struct PostController {
    service: PostService,
}

#[controller(route = "/posts")]
impl PostController {
    #[post]
    async fn create(
        &self,
        ValidatedJson(body): ValidatedJson<PostInput>,
    ) -> HttpResult<Created<Json<Post>>> {
        let post = self
            .service
            .create(body.title, body.body)
            .await
            .map_err(InternalError::new)?;
        Ok(Created(Json(post)))
    }

    #[get]
    async fn list(&self) -> HttpResult<Json<Vec<Post>>> {
        Ok(Json(self.service.list().await.map_err(InternalError::new)?))
    }

    #[get("/:id")]
    async fn find(&self, Path(id): Path<i32>) -> HttpResult<Json<Post>> {
        self.service
            .find(id)
            .await
            .map_err(InternalError::new)?
            .map(Json)
            .ok_or_else(|| NotFound::new("post not found").into())
    }

    #[put("/:id")]
    async fn update(
        &self,
        Path(id): Path<i32>,
        ValidatedJson(body): ValidatedJson<PostInput>,
    ) -> HttpResult<Json<Post>> {
        self.service
            .update(id, body.title, body.body)
            .await
            .map_err(InternalError::new)?
            .map(Json)
            .ok_or_else(|| NotFound::new("post not found").into())
    }

    #[delete("/:id")]
    async fn delete(&self, Path(id): Path<i32>) -> HttpResult<NoContent> {
        if self.service.delete(id).await.map_err(InternalError::new)? {
            Ok(NoContent)
        } else {
            Err(NotFound::new("post not found").into())
        }
    }
}
