use unicode_segmentation::UnicodeSegmentation;

pub struct NewUser {
    pub email: UserEmail,
    pub name: UserName,
}

#[derive(Debug, Default)]
pub struct UserName(String);

#[derive(Debug, Default)]
pub struct UserEmail(String);

impl AsRef<str> for UserName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for UserEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// TODO: This is a dummy implementation
impl TryFrom<String> for UserEmail {
    type Error = String;
    fn try_from(email: String) -> Result<Self, Self::Error> {
        if !email.contains('@') {
            return Err("email must contain an @".into());
        }
        Ok(Self(email))
    }
}

// TODO: This is a dummy implementation
impl TryFrom<String> for UserName {
    type Error = String;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        if name.trim().is_empty() {
            return Err("name must not be empty".into());
        }

        if name.graphemes(true).count() > 256 {
            return Err("name is too long".into());
        }

        let forbidden_chars = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let has_forbidden_chars = name.chars().any(|x| forbidden_chars.contains(&x));

        if has_forbidden_chars {
            return Err("name contains forbidden characters".into());
        }

        Ok(Self(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fake::faker::internet::en::SafeEmail;
    use fake::Fake;

    #[test]
    fn valid_emails_are_parsed_successfully() {
        let email = SafeEmail().fake::<String>();
        assert!(UserEmail::try_from(email).is_ok());
    }
}
