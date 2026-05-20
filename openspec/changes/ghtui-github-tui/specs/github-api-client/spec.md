## ADDED Requirements

### Requirement: Auth token resolution
The system SHALL resolve GitHub authentication token in priority order: `GITHUB_TOKEN` env var, `GH_TOKEN` env var, `gh auth token` command output. If none found, system SHALL exit with clear error message.

#### Scenario: Token from environment
- **WHEN** `GITHUB_TOKEN` env var is set
- **THEN** system uses that token for all API requests

#### Scenario: Token from gh CLI
- **WHEN** no env var is set but `gh` CLI is installed and authenticated
- **THEN** system runs `gh auth token` and uses the output

#### Scenario: No token found
- **WHEN** no env var is set and `gh` CLI is not installed or not authenticated
- **THEN** system exits with error: "No GitHub token found. Set GITHUB_TOKEN or run 'gh auth login'"

### Requirement: API request handling
The system SHALL make authenticated HTTP requests to GitHub REST API v3 with proper headers (Authorization, Accept, User-Agent).

#### Scenario: Successful API request
- **WHEN** system makes an API request with valid token
- **THEN** request includes `Authorization: Bearer <token>` header and receives JSON response

#### Scenario: API error response
- **WHEN** GitHub API returns 4xx/5xx status
- **THEN** system extracts error message from response body and surfaces it to the UI

### Requirement: Rate limit tracking
The system SHALL track GitHub API rate limit state from response headers and display remaining quota in the UI.

#### Scenario: Rate limit headers parsed
- **WHEN** API response includes `X-RateLimit-Remaining` and `X-RateLimit-Reset` headers
- **THEN** system stores these values and shows remaining requests in status bar

#### Scenario: Rate limit exceeded
- **WHEN** `X-RateLimit-Remaining` reaches 0
- **THEN** system shows "Rate limited - resets in {N}s" and queues requests until reset time

### Requirement: Response caching
The system SHALL cache API responses in memory with configurable TTL per resource type.

#### Scenario: Cache hit
- **WHEN** system requests a resource that was fetched within its TTL window
- **THEN** system returns cached data without making an API call

#### Scenario: Cache miss
- **WHEN** system requests a resource not in cache or expired
- **THEN** system fetches from API, stores in cache with TTL, returns fresh data

#### Scenario: Force refresh
- **WHEN** user presses `r` to refresh a panel
- **THEN** system bypasses cache and fetches fresh data from API

### Requirement: Pagination
The system SHALL handle paginated API responses, fetching subsequent pages as needed.

#### Scenario: Paginated results
- **WHEN** API response includes `Link` header with `rel="next"`
- **THEN** system fetches subsequent pages and concatenates results up to configured limit
