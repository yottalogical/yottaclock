use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index {
    pub user_id: Option<i32>,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct Login {}
