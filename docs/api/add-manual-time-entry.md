# Add manual time entry

> Source: https://kanbanflow.com/api-docs/add-manual-time-entry
> Scraped: 2026-07-30

# API Documentation

* * *

## Add manual time entry

### Description

Adds a new manual time entry. Returns an object containing the created manual time entry's ID.

### Request format

curl -X POST https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/manual-time-entries -H "Content-type: application/json" -d '{ "<PROPERTY\_1>": <PROPERTY\_VALUE\_1>, "<PROPERTY\_2>": <PROPERTY\_VALUE\_2>, ... "<PROPERTY\_N>": <PROPERTY\_VALUE\_N> }'

### Example request

curl -X POST https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/manual-time-entries -H "Content-type: application/json" -d '{ "startTimestamp": "2023-01-02T08:30:00Z", "endTimestamp": "2023-01-02T12:00:00Z" }'

### Example response

{
    "\_id": "EyS84FdLk"
}

### Valid properties

| Property | Type | Comment |
| --- | --- | --- |
| userId | String | Optional. The ID of the user that this entry should be associated with. See **Get users** tab for information about how to get a user's ID. |
| startTimestamp | String | Required. The UTC timestamp when the manual time entry started, for example 2023-12-31T09:00:00Z. |
| endTimestamp | String | Required. The UTC timestamp when the manual time entry ended, for example 2023-12-31T17:00:00Z. |
| comment | String | Optional. A comment to the time entry. Limited to 50 characters. |
| labelNames | Array | Optional. Labels for the time entry. Array of strings. Example: \["Billable", "Project X"\]. |
