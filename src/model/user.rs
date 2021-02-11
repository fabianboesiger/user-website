use crate::{
    database::get_pool,
    error::Error,
    regexes::{PASSWORD_LENGTH, USERNAME_CHARS, USERNAME_LENGTH},
};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2
};
use rand::rngs::OsRng;
use std::collections::HashMap;
use std::fmt;

pub struct User {
    pub username: String,
    pub password: String,
}

pub enum Signup {
    UsernameTaken,
}

impl fmt::Display for Signup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::UsernameTaken => "Dieser Benutzername ist bereits vergeben.",
            }
        )
    }
}

pub enum Signin {
    UserDoesNotExist,
    InvalidPassword,
}

impl fmt::Display for Signin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::UserDoesNotExist => "Dieser Benutzer scheint nicht zu existieren.",
                Self::InvalidPassword => "Das Passwort ist nicht korrekt.",
            }
        )
    }
}

pub enum FromForm {
    UsernameInvalidChars,
    UsernameInvalidLength,
    PasswordInvalidLength,
    PasswordsDiffer,
}

impl fmt::Display for FromForm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::UsernameInvalidChars => "Der Benutzername enthält ungültige Zeichen.",
                Self::UsernameInvalidLength =>
                    "Der Benutzername muss zwischen 2 und 16 Zeichen lang sein.",
                Self::PasswordInvalidLength =>
                    "Das Passwort muss zwischen 4 und 32 Zeichen lang sein.",
                Self::PasswordsDiffer =>
                    "Die Passwörter stimmen nicht überein.",
            }
        )
    }
}

impl User {
    pub fn from_signup_form(mut form: HashMap<String, String>) -> Result<User, Vec<FromForm>> {
        let username = form.remove("username").unwrap_or(String::new());
        let password = form.remove("password").unwrap_or(String::new());
        let confirm_password = form.remove("confirm-password").unwrap_or(String::new());

        let mut errors = Vec::new();

        if !USERNAME_CHARS.is_match(&username) {
            errors.push(FromForm::UsernameInvalidChars);
        }

        if !USERNAME_LENGTH.is_match(&username) {
            errors.push(FromForm::UsernameInvalidLength);
        }

        if !PASSWORD_LENGTH.is_match(&password) {
            errors.push(FromForm::PasswordInvalidLength);
        }

        if password != confirm_password {
            errors.push(FromForm::PasswordsDiffer);
        }

        if errors.is_empty() {
            Ok(User { username, password })
        } else {
            Err(errors)
        }
    }

    pub fn from_signin_form(mut form: HashMap<String, String>) -> Result<User, Vec<FromForm>> {
        let username = form.remove("username").unwrap_or(String::new());
        let password = form.remove("password").unwrap_or(String::new());
        let mut errors = Vec::new();

        if !USERNAME_CHARS.is_match(&username) {
            errors.push(FromForm::UsernameInvalidChars);
        }

        if !USERNAME_LENGTH.is_match(&username) {
            errors.push(FromForm::UsernameInvalidLength);
        }

        if !PASSWORD_LENGTH.is_match(&password) {
            errors.push(FromForm::PasswordInvalidLength);
        }

        if errors.is_empty() {
            Ok(User { username, password })
        } else {
            Err(errors)
        }
    }

    pub async fn signup(&self) -> Result<Result<User, Vec<Signup>>, Error> {
        let password = self.password.clone();
        let password = tokio::task::spawn_blocking(move || {
            let salt = SaltString::generate(&mut OsRng);
            Argon2::default()
                .hash_password_simple(password.as_bytes(), salt.as_ref())
                .unwrap()
                .to_string()
        })
        .await
        .unwrap();
        
        let user = sqlx::query_as!(
            User,
            "INSERT INTO users (username, password)
            VALUES ($1, $2)
            RETURNING username, password",
            self.username,
            password
        )
        .fetch_one(get_pool())
        .await;

        match user {
            Ok(user) => Ok(Ok(user)),
            Err(err) => {
                if let sqlx::Error::Database(err) = &err {
                    if let Some(code) = err.code() {
                        if code == "23505" {
                            return Ok(Err(vec![Signup::UsernameTaken]));
                        }
                    }
                }
                Err(err.into())
            }
        }
    }

    pub async fn signin(&self) -> Result<Result<User, Vec<Signin>>, Error> {
        let mut users = sqlx::query_as!(
            User,
            "SELECT *
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
                Ok(Err(vec![Signin::InvalidPassword]))
            }
        } else {
            Ok(Err(vec![Signin::UserDoesNotExist]))
        }
    }
}
