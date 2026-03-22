use super::*;

pub(crate) fn github_repo_slug_for_repo(repo_root: &Path) -> Option<String> {
    let remote_url = git_origin_remote_url(repo_root)?;
    github_repo_slug_from_remote_url(remote_url.trim())
}

pub(crate) fn github_avatar_url_for_repo_slug(repo_slug: &str) -> Option<String> {
    let (owner, _) = repo_slug.split_once('/')?;
    Some(format!(
        "https://avatars.githubusercontent.com/{owner}?size=96"
    ))
}

pub(crate) fn github_repo_url(repo_slug: &str) -> String {
    format!("https://github.com/{repo_slug}")
}

pub(crate) fn github_authenticated_user(
    saved_token: Option<&str>,
) -> Option<(String, Option<String>)> {
    let token = resolve_github_access_token(saved_token)?;
    let response = ureq::get("https://api.github.com/user")
        .header("Authorization", &format!("Bearer {token}"))
        .header("User-Agent", "Arbor")
        .call()
        .ok()?;

    if response.status() != 200 {
        return None;
    }

    let body = response.into_body().read_to_string().ok()?;
    let payload = serde_json::from_str::<serde_json::Value>(&body).ok()?;
    let login = payload
        .get("login")
        .and_then(|value| value.as_str())
        .and_then(non_empty_trimmed_str)
        .map(str::to_owned)?;
    let avatar_url = payload
        .get("avatar_url")
        .and_then(|value| value.as_str())
        .and_then(non_empty_trimmed_str)
        .map(str::to_owned);

    Some((login, avatar_url))
}

pub(crate) fn git_origin_remote_url(repo_root: &Path) -> Option<String> {
    let repo = gix::open(repo_root).ok()?;
    let remote = repo.find_remote("origin").ok()?;
    let url = remote.url(gix::remote::Direction::Fetch)?;
    let url_str = url.to_bstring().to_string();
    if url_str.is_empty() {
        return None;
    }
    Some(url_str)
}

pub(crate) fn github_repo_slug_from_remote_url(remote_url: &str) -> Option<String> {
    if let Some(path) = remote_url.strip_prefix("git@github.com:") {
        return github_repo_slug_from_path(path);
    }

    if let Some(path) = remote_url.strip_prefix("https://github.com/") {
        return github_repo_slug_from_path(path);
    }

    if let Some(path) = remote_url.strip_prefix("http://github.com/") {
        return github_repo_slug_from_path(path);
    }

    if let Some(path) = remote_url.strip_prefix("ssh://git@github.com/") {
        return github_repo_slug_from_path(path);
    }

    None
}

pub(crate) fn github_repo_slug_from_path(path: &str) -> Option<String> {
    let normalized = path.trim_end_matches('/');
    let repository_path = normalized.strip_suffix(".git").unwrap_or(normalized);
    let (owner, repository) = repository_path.split_once('/')?;
    if owner.is_empty() || repository.is_empty() {
        return None;
    }

    Some(format!("{owner}/{repository}"))
}

pub(crate) fn github_pr_number_for_worktree(
    github_service: &dyn github_service::GitHubService,
    worktree_path: &Path,
    branch: &str,
    github_token: Option<&str>,
) -> Option<u64> {
    if branch.trim().is_empty() || branch == "-" {
        return None;
    }

    github_pr_number_by_tracking_branch(github_service, worktree_path, github_token).or_else(|| {
        github_pr_number_by_head_branch(github_service, worktree_path, branch, github_token)
    })
}

pub(crate) fn should_lookup_pull_request_for_worktree(worktree: &WorktreeSummary) -> bool {
    if worktree.is_primary_checkout {
        return false;
    }

    let branch = worktree.branch.as_str();
    if branch == "-" || branch.is_empty() {
        return false;
    }

    !(branch.eq_ignore_ascii_case("main")
        || branch.eq_ignore_ascii_case("master")
        || branch.eq_ignore_ascii_case("develop")
        || branch.eq_ignore_ascii_case("dev")
        || branch.eq_ignore_ascii_case("trunk"))
}

pub(crate) fn github_pr_number_by_tracking_branch(
    github_service: &dyn github_service::GitHubService,
    worktree_path: &Path,
    github_token: Option<&str>,
) -> Option<u64> {
    let branch = git_branch_name_for_worktree(worktree_path).ok()?;
    github_pr_number_by_head_branch(github_service, worktree_path, &branch, github_token)
}

pub(crate) fn github_pr_number_by_head_branch(
    github_service: &dyn github_service::GitHubService,
    worktree_path: &Path,
    branch: &str,
    github_token: Option<&str>,
) -> Option<u64> {
    let slug = github_repo_slug_for_repo(worktree_path)?;
    let token = resolve_github_access_token(github_token)?;
    github_service.open_pull_request_number(&slug, branch, &token)
}

pub(crate) fn github_pr_url(repo_slug: &str, pr_number: u64) -> String {
    format!("https://github.com/{repo_slug}/pull/{pr_number}")
}

pub(crate) fn non_empty_trimmed_str(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

pub(crate) fn github_access_token_from_env() -> Option<String> {
    env::var("GITHUB_TOKEN")
        .ok()
        .and_then(|value| non_empty_trimmed_str(&value).map(str::to_owned))
}

pub(crate) fn resolve_github_access_token(saved_token: Option<&str>) -> Option<String> {
    let env_token = github_access_token_from_env();
    resolve_github_access_token_from_sources(saved_token, env_token.as_deref())
        .or_else(github_service::github_access_token_from_gh_cli)
}

pub(crate) fn resolve_github_access_token_from_sources(
    saved_token: Option<&str>,
    env_token: Option<&str>,
) -> Option<String> {
    saved_token
        .and_then(non_empty_trimmed_str)
        .map(str::to_owned)
        .or_else(|| env_token.and_then(non_empty_trimmed_str).map(str::to_owned))
}

pub(crate) fn github_oauth_client_id() -> Option<String> {
    env::var("ARBOR_GITHUB_OAUTH_CLIENT_ID")
        .ok()
        .or_else(|| env::var("GITHUB_OAUTH_CLIENT_ID").ok())
        .or_else(|| BUILT_IN_GITHUB_OAUTH_CLIENT_ID.map(str::to_owned))
        .and_then(|value| non_empty_trimmed_str(&value).map(str::to_owned))
}

/// Detect the web URL for any supported provider (GitHub, Azure DevOps, GitLab).
pub(crate) fn repo_web_url_for_repo(repo_root: &Path) -> Option<String> {
    let remote_url = git_origin_remote_url(repo_root)?;
    let trimmed = remote_url.trim();

    // GitHub
    if let Some(slug) = github_repo_slug_from_remote_url(trimmed) {
        return Some(github_repo_url(&slug));
    }

    // Azure DevOps
    if let Some(url) = azure_devops_web_url_from_remote(trimmed) {
        return Some(url);
    }

    None
}

/// Parse an Azure DevOps remote URL and return the web URL for the repo.
pub(crate) fn azure_devops_web_url_from_remote(remote_url: &str) -> Option<String> {
    let spec = azure_devops_spec_from_remote(remote_url)?;
    Some(format!(
        "https://dev.azure.com/{}/{}/{}/{}",
        spec.0, spec.1, "_git", spec.2
    ))
}

/// Extract (org, project, repo) from an Azure DevOps remote URL.
fn azure_devops_spec_from_remote(remote_url: &str) -> Option<(String, String, String)> {
    // HTTPS: https://dev.azure.com/{org}/{project}/_git/{repo}
    if let Some(path) = remote_url
        .strip_prefix("https://dev.azure.com/")
        .or_else(|| remote_url.strip_prefix("http://dev.azure.com/"))
    {
        return parse_ado_https_path(path);
    }

    // Legacy HTTPS: https://{org}.visualstudio.com/{project}/_git/{repo}
    if let Some(rest) = remote_url.strip_prefix("https://")
        && let Some((authority, path)) = rest.split_once('/')
        && let Some(org) = authority.strip_suffix(".visualstudio.com")
        && !org.is_empty()
    {
        return parse_ado_https_path(&format!("{org}/{path}"));
    }

    // SSH: git@ssh.dev.azure.com:v3/{org}/{project}/{repo}
    if let Some(path) = remote_url.strip_prefix("git@ssh.dev.azure.com:v3/") {
        return parse_ado_ssh_path(path);
    }

    // Legacy SSH: {org}@vs-ssh.visualstudio.com:v3/{org}/{project}/{repo}
    if let Some((_, path)) = remote_url.split_once("vs-ssh.visualstudio.com:v3/") {
        return parse_ado_ssh_path(path);
    }

    None
}

fn parse_ado_https_path(path: &str) -> Option<(String, String, String)> {
    let (before_git, repo) = path.split_once("/_git/")?;
    let (org, project) = before_git.split_once('/')?;
    let repo = repo.trim_end_matches('/').trim_end_matches(".git");
    if org.is_empty() || project.is_empty() || repo.is_empty() {
        return None;
    }
    Some((org.to_owned(), project.to_owned(), repo.to_owned()))
}

fn parse_ado_ssh_path(path: &str) -> Option<(String, String, String)> {
    let mut parts = path.splitn(3, '/');
    let org = parts.next().filter(|s| !s.is_empty())?;
    let project = parts.next().filter(|s| !s.is_empty())?;
    let repo = parts.next().filter(|s| !s.is_empty())?;
    let repo = repo.trim_end_matches('/').trim_end_matches(".git");
    if repo.is_empty() {
        return None;
    }
    Some((org.to_owned(), project.to_owned(), repo.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_token_resolution_prefers_saved_token() {
        let token =
            resolve_github_access_token_from_sources(Some(" saved-token "), Some("env-token"));
        assert_eq!(token.as_deref(), Some("saved-token"));
    }

    #[test]
    fn github_token_resolution_falls_back_to_environment_token() {
        let token = resolve_github_access_token_from_sources(Some(""), Some(" env-token "));
        assert_eq!(token.as_deref(), Some("env-token"));
    }

    #[test]
    fn azure_devops_https_url_parsed() {
        let url =
            azure_devops_web_url_from_remote("https://dev.azure.com/myorg/myproject/_git/myrepo");
        assert_eq!(
            url.as_deref(),
            Some("https://dev.azure.com/myorg/myproject/_git/myrepo")
        );
    }

    #[test]
    fn azure_devops_ssh_url_parsed() {
        let url =
            azure_devops_web_url_from_remote("git@ssh.dev.azure.com:v3/myorg/myproject/myrepo");
        assert_eq!(
            url.as_deref(),
            Some("https://dev.azure.com/myorg/myproject/_git/myrepo")
        );
    }

    #[test]
    fn azure_devops_legacy_https_url_parsed() {
        let url = azure_devops_web_url_from_remote(
            "https://myorg.visualstudio.com/myproject/_git/myrepo",
        );
        assert_eq!(
            url.as_deref(),
            Some("https://dev.azure.com/myorg/myproject/_git/myrepo")
        );
    }

    #[test]
    fn azure_devops_legacy_ssh_url_parsed() {
        let url = azure_devops_web_url_from_remote(
            "myorg@vs-ssh.visualstudio.com:v3/myorg/myproject/myrepo",
        );
        assert_eq!(
            url.as_deref(),
            Some("https://dev.azure.com/myorg/myproject/_git/myrepo")
        );
    }

    #[test]
    fn azure_devops_returns_none_for_github_url() {
        let url = azure_devops_web_url_from_remote("https://github.com/owner/repo");
        assert!(url.is_none());
    }
}
