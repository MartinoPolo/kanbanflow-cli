# Rate limiting

> Source: https://kanbanflow.com/api-docs/rate-limiting
> Scraped: 2026-07-30

# API Documentation

* * *

## Rate limiting

You can make up to 1000 requests per hour and board. If you make more than 5000 requests in one day, the API token will be locked. The response headers contain information about how many requests you have left and when your current quota will reset.

| Header name | Description |
| --- | --- |
| X-RateLimit-Limit | The maximum number of requests you are allowed to make in the current rate limit window. |
| X-RateLimit-Remaining | How many requests you have left in the current rate limit window. |
| X-RateLimit-Reset | The time at which the current rate limit window will be reset in UTC epoch seconds. |
