use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index {
    pub user_id: Option<i32>,
}
