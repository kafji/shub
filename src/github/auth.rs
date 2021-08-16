use super::client::Authentication;

#[derive(PartialEq, Debug)]
pub struct PersonalAccessToken<'a> {
    username: &'a str,
    token: &'a str,
}

impl<'a> PersonalAccessToken<'a> {
    pub const fn new(username: &'a str, token: &'a str) -> Self {
        Self { username, token }
    }
}

impl Authentication for PersonalAccessToken<'_> {
    fn to_authz_value(&self) -> String {
        let pair = format!("{}:{}", &self.username, &self.token);
        let encoded = base64::encode(pair);
        format!("Basic {}", encoded)
    }
}
