# shipit - Stop cloning in your CI!

`shipit` is a tool for committing changes to JSON/YAML files from CI environments to supported Git providers.

## Why?

GitOps-enabled environments use git repositories to store deployment manifests. It's often wanted to have the CI update those repositories, but a lot of time is wasted in the whole "git clone/commit/push" ordeal. `shipit` uses REST APIs available in certain Git providers (eg. GitHub, GitLab, see below for the full list of supported APIs) to make the process super snappy!

## Support table

### Git Providers

*(TODO) means it's not supported, but coming soon!*

- (TODO) [Azure DevOps]
- (TODO) [BitBucket] (only BitBucket cloud ie. bitbucket.org)
- (TODO) [Gitea]
- (TODO) [GitHub] (both GitHub.com and GitHub Enterprise Server)
- (TODO) [GitLab] (both self-managed and gitlab.com)

### Templaters

- [JSON Patch](https://jsonpatch.com/) for JSON files

[azure devops]: https://azure.microsoft.com/en-us/services/devops/repos/
[gitlab]: https://gitlab.com
[github]: https://github.com
[gitea]: https://gitea.com
[bitbucket]: https://bitbucket.com
