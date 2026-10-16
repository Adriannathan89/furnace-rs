use furnace_rs::prelude::*;
use furnace_rs_persistence::storage::{Page, StorageError, StorageErrorKind};
use sea_orm::ActiveValue::Set;

use super::{
    model::{Post, PostInput, PostPagination, entity},
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
        let post = self
            .repository
            .entity()
            .insert(entity::ActiveModel {
                title: Set(body.title),
                body: Set(body.body),
                ..Default::default()
            })
            .await?;
        Ok(Created(Json(post.into())))
    }

    #[get]
    async fn list(&self, Query(page): Query<PostPagination>) -> HttpResult<Json<Page<Post>>> {
        let page = self
            .repository
            .entity()
            .query()
            .order_by_desc(entity::Column::Id)
            .paginate(page.page_number, page.page_size)
            .await?;
        Ok(Json(Page {
            items: page.items.into_iter().map(Into::into).collect(),
            page_number: page.page_number,
            page_size: page.page_size,
        }))
    }

    #[get("/:id")]
    async fn find(&self, Path(id): Path<i32>) -> HttpResult<Json<Post>> {
        self.repository
            .entity()
            .find_by_id(id)
            .await?
            .map(|post| Json(post.into()))
            .ok_or_else(|| NotFound::new("post not found").into())
    }

    #[put("/:id")]
    async fn update(
        &self,
        Path(id): Path<i32>,
        ValidatedJson(body): ValidatedJson<PostInput>,
    ) -> HttpResult<Json<Post>> {
        let mut active = self
            .repository
            .entity()
            .find_by_id(id)
            .await?
            .map(Into::<entity::ActiveModel>::into)
            .ok_or_else(|| NotFound::new("post not found"))?;
        active.title = Set(body.title);
        active.body = Set(body.body);
        let post = self
            .repository
            .entity()
            .update(active)
            .await
            .map_err(storage_error)?;
        Ok(Json(post.into()))
    }

    #[delete("/:id")]
    async fn delete(&self, Path(id): Path<i32>) -> HttpResult<NoContent> {
        if self.repository.entity().delete_by_id(id).await? {
            Ok(NoContent)
        } else {
            Err(NotFound::new("post not found").into())
        }
    }
}

fn storage_error(error: StorageError) -> HttpError {
    match error.kind() {
        StorageErrorKind::InvalidPagination => {
            HttpError::bad_request("page_number and page_size must be positive")
        }
        StorageErrorKind::NotFound => HttpError::not_found("post not found"),
        StorageErrorKind::MissingOrder | StorageErrorKind::Database => HttpError::internal(error),
        _ => HttpError::internal(error),
    }
}
