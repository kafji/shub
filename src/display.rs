use crate::{
    github_models::{GhCheckRun, GhCommit},
    OwnedRepository, StarredRepository,
};
use bstr::BStr;
use chrono::{DateTime, TimeZone, Utc};
use octocrab::models::Repository;
use std::{borrow::Cow, fmt};
use unicode_segmentation::UnicodeSegmentation;

macro_rules! write_col {
    ($w:expr, $len:expr, $txt:expr) => {
        write!($w, "{:len$}", ellipsize($txt, $len as _), len = $len as _)
    };
    (, $w:expr, $len:expr, $txt:expr) => {
        write!(
            $w,
            " | {:len$}",
            ellipsize($txt, $len as _),
            len = $len as _
        )
    };
    ($w:expr, $len:expr, $txt:expr, ) => {
        write!(
            $w,
            "{:len$} | ",
            ellipsize($txt, $len as _),
            len = $len as _
        )
    };
    (, $w:expr, $len:expr, $txt:expr, ) => {
        write!(
            $w,
            " | {:len$} | ",
            ellipsize($txt, $len as _),
            len = $len as _
        )
    };
}

const OWNER_NAME_LEN: u8 = 15;
const COMMIT_MSG_LEN: u8 = 40;
const LANG_NAME_LEN: u8 = 10;
const PUSHED_AT_LEN: u8 = 12;

pub fn ellipsize(text: &str, threshold: usize) -> Cow<'_, str> {
    debug_assert!(threshold > 2);
    if text.len() <= threshold {
        text.into()
    } else {
        let text: String = text
            .chars()
            .map(|c| if c == '\n' { ' ' } else { c })
            .take(threshold - 2)
            .collect();
        let text: String = text.trim().chars().chain("..".chars()).collect();
        text.into()
    }
}

#[cfg(test)]
#[test]
fn test_ellipsize() {
    use quickcheck::{quickcheck, TestResult};

    fn has_max_length_threshold(text: String, threshold: usize) -> TestResult {
        if threshold < 3 {
            return TestResult::discard();
        }
        TestResult::from_bool(ellipsize(&text, threshold).chars().count() <= threshold)
    }

    quickcheck(has_max_length_threshold as fn(_, _) -> TestResult);

    fn has_ellipsis_at_the_end(text: String, threshold: usize) -> TestResult {
        if threshold < 3 {
            return TestResult::discard();
        }
        if text.chars().count() <= threshold {
            return TestResult::discard();
        }
        let ellipsized = ellipsize(&text, threshold);
        TestResult::from_bool(ellipsized.ends_with("..."))
    }

    quickcheck(has_ellipsis_at_the_end as fn(_, _) -> TestResult);
}

/// Relative time from now.
pub trait RelativeTime {
    fn since(&self) -> Since;
}

impl<T> RelativeTime for DateTime<T>
where
    T: TimeZone,
{
    fn since(&self) -> Since {
        let duration = Utc::now().signed_duration_since(self.clone());
        Since(duration)
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub struct Since(chrono::Duration);

impl fmt::Display for Since {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let days = self.0.num_days();
        match days {
            _ if days < 1 => {
                let hours = self.0.num_hours();
                if hours < 1 {
                    let minutes = self.0.num_minutes();
                    if minutes < 1 {
                        write!(f, "just now")
                    } else {
                        write!(f, "{minutes} minutes ago")
                    }
                } else {
                    write!(f, "{hours} hours ago")
                }
            }
            _ if days < 7 => {
                write!(f, "this week")
            }
            _ if days < 30 => {
                write!(f, "this month")
            }
            _ if days < 365 => {
                write!(f, "this year")
            }
            _ => {
                let years = days / 365;
                if years == 1 {
                    write!(f, "{years} year ago")
                } else {
                    write!(f, "{years} years ago")
                }
            }
        }
    }
}

#[derive(Debug)]
struct RepositoryName<'a>(&'a str);

impl<'a> From<&'a Repository> for RepositoryName<'a> {
    fn from(s: &'a Repository) -> Self {
        Self(&s.name)
    }
}

impl fmt::Display for RepositoryName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_col!(f, 15, self.0)?;
        Ok(())
    }
}

#[derive(Debug)]
struct RepositoryAttrs(String);

impl From<&Repository> for RepositoryAttrs {
    fn from(s: &Repository) -> Self {
        let mut attrs = Vec::new();

        if let Some(true) = s.archived {
            attrs.push("archived");
        }

        if let Some(true) = s.fork {
            attrs.push("fork");
        }

        let attrs = attrs
            .into_iter()
            .map(|x| ellipsize(x, 10))
            .collect::<Vec<_>>()
            .join(", ");
        Self(attrs)
    }
}

impl fmt::Display for RepositoryAttrs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_col!(f, 15, &self.0)?;
        Ok(())
    }
}

#[derive(Debug)]
struct RepositoryDescription<'a>(&'a str, usize);

impl<'a> RepositoryDescription<'a> {
    fn from_repository(repository: &'a Repository, length: usize) -> Self {
        let desc = repository.description.as_deref().unwrap_or_default();
        Self(desc, length)
    }
}

impl fmt::Display for RepositoryDescription<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_col!(f, self.1, self.0)?;
        Ok(())
    }
}

impl fmt::Display for OwnedRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repo = &self.0;
        let commit = &self.1;

        let visibility = repo
            .private
            .map(|x| if x { "private" } else { "public" })
            .unwrap_or_default();
        write_col!(f, 6, visibility,)?;

        let name: RepositoryName = repo.into();
        write!(f, "{}", name)?;

        let desc = RepositoryDescription::from_repository(repo, 30);
        write!(f, " | {}", &desc.to_string())?;

        let pushed = repo
            .pushed_at
            .as_ref()
            .map(|x| x.since().to_string())
            .map(Cow::Owned)
            .unwrap_or_default();
        write_col!(, f, PUSHED_AT_LEN, &pushed)?;

        let last_commit = commit
            .as_ref()
            .map(|x| &x.commit)
            .map(|x| x.message.as_str())
            .unwrap_or_default();
        write_col!(, f, COMMIT_MSG_LEN, last_commit)?;

        let lang = repo
            .language
            .as_ref()
            .and_then(|x| x.as_str())
            .unwrap_or_default();
        write_col!(, f, LANG_NAME_LEN, lang, )?;

        let attrs: RepositoryAttrs = repo.into();
        write!(f, "{}", attrs)?;

        Ok(())
    }
}

impl fmt::Display for StarredRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repo = &self.0;

        let name: RepositoryName = repo.into();
        write!(f, "{}", name)?;

        let desc = RepositoryDescription::from_repository(repo, 60);
        write!(f, " | {}", &desc.to_string())?;

        let owner = repo
            .owner
            .as_ref()
            .map(|x| x.login.as_str())
            .unwrap_or_default();
        write_col!(, f, OWNER_NAME_LEN, owner)?;

        let pushed = repo
            .pushed_at
            .as_ref()
            .map(|x| x.since().to_string())
            .map(Cow::Owned)
            .unwrap_or_default();
        write_col!(, f, PUSHED_AT_LEN, &pushed)?;

        let lang = repo
            .language
            .as_ref()
            .and_then(|x| x.as_str())
            .unwrap_or_default();
        write_col!(, f, LANG_NAME_LEN, lang, )?;

        let attrs: RepositoryAttrs = repo.into();
        write!(f, "{}", attrs)?;

        Ok(())
    }
}

/// Transform `snake_case` to `Statement`.
///
/// Assumes characters are ASCII characters.
fn snake_case_to_statement(text: impl Into<String>) -> String {
    let s = text.into();
    let chars = s.chars().into_iter();
    let chars = chars.enumerate().map(|(i, c)| {
        if i == 0 {
            c.to_ascii_uppercase()
        } else if c == '_' {
            ' '
        } else {
            c
        }
    });
    chars.collect()
}

#[cfg(test)]
#[test]
fn test_snake_case_to_statement() {
    let input = "hello_world";
    let output = snake_case_to_statement(input);
    assert_eq!("Hello world", output);
}

#[derive(PartialEq, Clone, Debug)]
pub struct CommitInfo<'a> {
    pub author_name: Option<&'a str>,
    pub author_email: Option<&'a str>,
    pub timestamp: &'a DateTime<Utc>,
    pub hash: &'a BStr,
    pub message: &'a str,
}

impl<'a> CommitInfo<'a> {
    pub fn from_github_commit(commit: &'a GhCommit) -> Self {
        let author_name = commit.commit.author.name.as_deref();
        let author_email = commit.commit.author.email.as_deref();
        let timestamp = &commit.commit.author.date;
        let hash = commit.sha.as_str().into();
        let message = &commit.commit.message;
        Self {
            author_name,
            author_email,
            timestamp,
            hash,
            message,
        }
    }
}

impl fmt::Display for CommitInfo<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(author_name) = self.author_name {
            write!(f, "{author_name}")?;
            if let Some(author_email) = self.author_email {
                write!(f, " <{author_email}>")?;
            }
            write!(f, " - ")?;
        } else if let Some(author_email) = self.author_email {
            write!(f, "{author_email} - ")?;
        }
        writeln!(f, "{}", self.timestamp.since())?;
        writeln!(f, "{}", &self.hash[..8])?;
        writeln!(
            f,
            "{}",
            self.message
                .graphemes(false)
                .take_while(|&x| x != "\n")
                .collect::<String>()
        )?;
        Ok(())
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct BuildsInfo<'a> {
    builds: Vec<BuildInfo<'a>>,
}

impl<'a> BuildsInfo<'a> {
    pub fn from_github_check_runs(runs: &'a [GhCheckRun]) -> Self {
        let builds = runs.iter().map(BuildInfo::from_github_check_run).collect();
        Self { builds }
    }
}

impl fmt::Display for BuildsInfo<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for build in &self.builds {
            writeln!(f, "{}", build)?;
        }
        Ok(())
    }
}

#[derive(PartialEq, Clone, Debug)]
struct BuildInfo<'a> {
    name: &'a str,
    status: &'a str,
    timestamp: &'a DateTime<Utc>,
}

impl<'a> BuildInfo<'a> {
    fn from_github_check_run(run: &'a GhCheckRun) -> Self {
        let name = &run.name;
        let status = run.conclusion.as_deref().unwrap_or(&run.status);
        let timestamp = run.completed_at.as_ref().unwrap_or(&run.started_at);
        Self {
            name,
            status,
            timestamp,
        }
    }
}

impl fmt::Display for BuildInfo<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} - {}",
            self.name,
            snake_case_to_statement(self.status),
            self.timestamp.since()
        )
    }
}
