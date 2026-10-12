use furnace_rs::prelude::*;

use super::{
    model::{Post, PostInput},
    repository::PostRepository,
};

#[controller]
pub struct PostController {
    #[inject]
    repository: PostRepository,
}

#[controller(route = "/posts")]
impl PostController {
    #[post]
    async fn create(
        &self,
        ValidatedJson(body): ValidatedJson<PostInput>,
    ) -> HttpResult<Created<Json<Post>>> {
        let post = self.repository.create(body.title, body.body).await?;
        Ok(Created(Json(post)))
    }

    #[get]
    async fn list(&self) -> HttpResult<Json<Vec<Post>>> {
        Ok(Json(self.repository.list().await?))
    }

    #[get("/:id")]
    async fn find(&self, Path(id): Path<i32>) -> HttpResult<Json<Post>> {
        self.repository
            .find(id)
            .await?
            .map(Json)
            .ok_or_else(|| NotFound::new("post not found").into())
    }

    #[put("/:id")]
    async fn update(
        &self,
        Path(id): Path<i32>,
        ValidatedJson(body): ValidatedJson<PostInput>,
    ) -> HttpResult<Json<Post>> {
        self.repository
            .update(id, body.title, body.body)
            .await?
            .map(Json)
            .ok_or_else(|| NotFound::new("post not found").into())
    }

    #[delete("/:id")]
    async fn delete(&self, Path(id): Path<i32>) -> HttpResult<NoContent> {
        if self.repository.delete(id).await? {
            Ok(NoContent)
        } else {
            Err(NotFound::new("post not found").into())
        }
    }
}
