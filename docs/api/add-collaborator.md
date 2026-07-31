# Add collaborator

> Source: https://kanbanflow.com/api-docs/add-collaborator
> Scraped: 2026-07-30

# API Documentation

* * *

## Add collaborator

### Description

Add a collaborator to a task. The collaborators are automatically ordered by name.

### Request format

curl -X POST https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/collaborators -H "Content-type: application/json" -d '{ "<PROPERTY\_1>": <PROPERTY\_VALUE\_1>, "<PROPERTY\_2>": <PROPERTY\_VALUE\_2>, ... "<PROPERTY\_N>": <PROPERTY\_VALUE\_N> }'

### Example request

curl -X POST https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/collaborators -H "Content-type: application/json"
 -d '{ "userId": "UHJ9JgtA" }'

### Valid properties

| Property | Type | Comment |
| --- | --- | --- |
| userId | String | Required. The ID of the user. See **Get users** tab for information about how to get a user's ID. |
