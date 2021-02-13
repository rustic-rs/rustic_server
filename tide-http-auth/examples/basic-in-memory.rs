use std::collections::HashMap;
use std::env;
use tide_http_auth::{BasicAuthRequest, Storage};

// We define our user struct like so:
#[derive(Clone)]
struct User {
    username: String,
    favorite_food: String,

    // We include the password here, which is not very secure. This is for
    // illustrative purposes only.
    password: String,
}

// We're creating an in-memory map of usernames to users.
#[derive(Clone)]
struct ExampleState {
    users: HashMap<String, User>,
}

impl ExampleState {
    pub fn new(userlist: Vec<User>) -> Self {
        let mut users = HashMap::new();
        for user in userlist {
            users.insert(user.username.to_owned(), user);
        }

        ExampleState { users }
    }
}

#[async_trait::async_trait]
impl Storage<User, BasicAuthRequest> for ExampleState {
    async fn get_user(&self, request: BasicAuthRequest) -> tide::Result<Option<User>> {
        match self.users.get(&request.username) {
            Some(user) => {
                // Again, this is just an example. In practice you'd want to use something called a
                // "constant time comparison function" to check if the passwords are equivalent to
                // avoid a timing attack.
                if user.password != request.password {
                    return Ok(None);
                }

                Ok(Some(user.clone()))
            }
            None => Ok(None),
        }
    }
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = env::var("PORT").ok().unwrap_or_else(|| "8080".to_string());
    let host = env::var("HOST")
        .ok()
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let addr = format!("{}:{}", host, port);

    let users = vec![
        User {
            username: "Basil".to_string(),
            favorite_food: "Cat food".to_string(),
            password: "cool meow time".to_string(),
        },
        User {
            username: "Fern".to_string(),
            favorite_food: "Human food".to_string(),
            password: "hunter2 am I doing this right".to_string(),
        },
    ];

    let mut app = tide::with_state(ExampleState::new(users));

    app.with(tide_http_auth::Authentication::new(
        tide_http_auth::BasicAuthScheme::default(),
    ));

    app.at("/").get(hello);

    println!(
        r#"
Listening at http://{}/. Open this URL in your browser and input one of the following:

Username: Basil
Password: cool meow time

Username: Fern
Password: hunter2 am I doing this right

"#,
        &addr
    );
    app.listen(addr).await?;

    Ok(())
}

async fn hello<State>(req: tide::Request<State>) -> tide::Result<tide::Response> {
    if let Some(user) = req.ext::<User>() {
        Ok(format!(
            "hi {}! your favorite food is {}.",
            user.username, user.favorite_food
        )
        .into())
    } else {
        let mut response: tide::Response = "howdy stranger".to_string().into();
        response.set_status(tide::http::StatusCode::Unauthorized);
        response.insert_header("WWW-Authenticate", "Basic");
        Ok(response)
    }
}
