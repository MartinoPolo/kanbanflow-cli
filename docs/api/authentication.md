# Authentication

> Source: https://kanbanflow.com/api-docs/authentication
> Scraped: 2026-07-30

# API Documentation

* * *

## Authentication

To use the API, you must first create an API token. This token must be included in all API requests.

To create a token, open **Menu** in the top right corner of a premium board. Then go to **Settings** → **API & Webhooks** and click **Add API token**.

The token is sent as a Bearer token in the _Authorization_ header.

### Request format

curl -H "Authorization: Bearer <SECRET\_TOKEN>" https://kanbanflow.com/api/v1/<RESOURCE>

### Example

curl -H "Authorization: Bearer 6FsUJdDqjyQj9Gfk29dLcgAJSM" https://kanbanflow.com/api/v1/board
