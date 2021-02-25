use crate::{
    database::get_pool,
    error::Error,
    regexes::{PASSWORD_LENGTH, USERNAME_CHARS, USERNAME_LENGTH},
};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::rngs::OsRng;
use std::collections::HashMap;
use std::fmt;

pub type UserId = i32;

async fn signup_action(user_id: UserId) -> Result<(), Error> {
    sqlx::query!(
        "INSERT INTO states (user_id)
        VALUES ($1)",
        user_id,
    )
    .execute(get_pool())
    .await?;

    Ok(())
}

pub enum Flash {
    UsernameTaken,
    UserDoesNotExist,
    InvalidPassword,
    UsernameInvalidChars,
    UsernameInvalidLength,
    PasswordInvalidLength,
    PasswordsDiffer,
}

impl fmt::Display for Flash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::UsernameTaken => "Dieser Benutzername ist bereits vergeben.",
                Self::UserDoesNotExist => "Dieser Benutzer scheint nicht zu existieren.",
                Self::InvalidPassword => "Das Passwort ist nicht korrekt.",
                Self::UsernameInvalidChars => "Der Benutzername enthält ungültige Zeichen.",
                Self::UsernameInvalidLength =>
                    "Der Benutzername muss zwischen 2 und 16 Zeichen lang sein.",
                Self::PasswordInvalidLength =>
                    "Das Passwort muss zwischen 4 und 32 Zeichen lang sein.",
                Self::PasswordsDiffer => "Die Passwörter stimmen nicht überein.",
            }
        )
    }
}

pub fn extract_username(form: &mut HashMap<String, String>) -> Result<String, Vec<Flash>> {
    let username = form.remove("username");
    let mut errors = Vec::new();

    if let Some(username) = &username {
        if !USERNAME_LENGTH.is_match(username) {
            errors.push(Flash::UsernameInvalidLength);
        }

        if !USERNAME_CHARS.is_match(username) {
            errors.push(Flash::UsernameInvalidChars);
        }
    } else {
        errors.push(Flash::UsernameInvalidLength);
    }

    if errors.is_empty() {
        Ok(username.unwrap())
    } else {
        Err(errors)
    }
}

pub fn extract_password(form: &mut HashMap<String, String>) -> Result<String, Vec<Flash>> {
    let password = form.remove("password");
    let mut errors = Vec::new();

    if let Some(password) = &password {
        if !PASSWORD_LENGTH.is_match(password) {
            errors.push(Flash::PasswordInvalidLength);
        }
    } else {
        errors.push(Flash::PasswordInvalidLength);
    }

    if errors.is_empty() {
        Ok(password.unwrap())
    } else {
        Err(errors)
    }
}

pub fn extract_confirm_password(form: &mut HashMap<String, String>) -> Result<String, Vec<Flash>> {
    let mut errors = Vec::new();

    if form.get("password") != form.get("confirm-password") {
        errors.push(Flash::PasswordsDiffer);
    }

    match extract_password(form) {
        Ok(ok) => {
            if errors.is_empty() {
                Ok(ok)
            } else {
                Err(errors)
            }
        }
        Err(mut err) => {
            errors.append(&mut err);
            Err(errors)
        }
    }
}

#[macro_export]
macro_rules! combine {
    ($n:ident { $( $k:ident : $v:expr ),* $(,)? }) => {{
        let mut errors = Vec::new();

        $(
            let $k = match $v {
                Ok(ok) => Some(ok),
                Err(mut err) => {
                    errors.append(&mut err);
                    None
                },
            };
        )*

        if errors.is_empty() {


            Ok($n {
                $(
                    $k: $k.unwrap(),
                )*
            })
        } else {
            Err(errors)
        }
    }};
}

pub async fn hash(password: String) -> String {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password_simple(password.as_bytes(), salt.as_ref())
            .unwrap()
            .to_string()
    })
    .await
    .unwrap()
}

pub struct User {
    pub username: String,
    pub password: String,
}

impl User {
    pub async fn signup(&self) -> Result<Result<User, Vec<Flash>>, Error> {
        let password = hash(self.password.clone()).await;

        let user = sqlx::query!(
            "INSERT INTO users (username, password)
            VALUES ($1, $2)
            RETURNING username, password, user_id",
            self.username,
            password,
        )
        .fetch_one(get_pool())
        .await;

        match user {
            Ok(user) => {
                signup_action(user.user_id).await?;

                let user = User {
                    username: user.username,
                    password: user.password,
                };


                Ok(Ok(user))
            },
            Err(err) => {
                if let sqlx::Error::Database(err) = &err {
                    log::warn!("{:?}", err);
                    if let Some(code) = err.code() {
                        if code == "23505" {
                            return Ok(Err(vec![Flash::UsernameTaken]));
                        }
                    }
                }
                Err(err.into())
            }
        }
    }

    pub async fn signin(&self) -> Result<Result<User, Vec<Flash>>, Error> {
        let mut users = sqlx::query_as!(
            User,
            "SELECT username, password
            FROM users
            WHERE username = $1",
            self.username
        )
        .fetch_all(get_pool())
        .await?;

        if let Some(user) = users.pop() {
            let password = self.password.clone();
            let hash = user.password.clone();
            let valid = tokio::task::spawn_blocking(move || {
                let parsed_hash = PasswordHash::new(&hash).unwrap();
                Argon2::default()
                    .verify_password(password.as_bytes(), &parsed_hash)
                    .is_ok()
            })
            .await
            .unwrap();

            if valid {
                Ok(Ok(user))
            } else {
                Ok(Err(vec![Flash::InvalidPassword]))
            }
        } else {
            Ok(Err(vec![Flash::UserDoesNotExist]))
        }
    }

    pub async fn delete(&self) -> Result<(), Error> {
        sqlx::query!(
            "DELETE FROM users
            WHERE username = $1",
            self.username,
        )
        .execute(get_pool())
        .await?;

        Ok(())
    }
}
