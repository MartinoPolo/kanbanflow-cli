# Get custom fields

> Source: https://kanbanflow.com/api-docs/get-board-custom-fields
> Scraped: 2026-07-30

# API Documentation

* * *

## Get custom fields

### Description

Returns the custom fields available on the board.

### Request format

curl https://kanbanflow.com/api/v1/custom-fields

### Example response

\[
    {
        "\_id": "VVH383",
        "name": "Project code",
        "fieldType": "text"
    },
    {
        "\_id": "gde6TS",
        "name": "Budget",
        "fieldType": "number",
        "numberSettings": {
            "prefix": "$"
        }
    },
    {
        "\_id": "c4tPlu",
        "name": "City",
        "fieldType": "dropdown",
        "dropdownOptions": \[
            {
                "text": "London"
            },
            {
                "text": "New York"
            }
        \]
    }
\]
