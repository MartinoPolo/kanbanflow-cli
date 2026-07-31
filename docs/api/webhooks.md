# Webhooks

> Source: https://kanbanflow.com/api-docs/webhooks
> Scraped: 2026-07-30

# API Documentation

* * *

## Webhooks

Webhooks allow you to get notified when changes are made on your board. We will send an HTTP POST request to an external URL with details of the event. You can configure up to 20 webhooks per board.

### Setup

You can administrate webhooks either in the **API & Webhooks** section of the board settings or via the API. For details on how to do it with the API, please refer to the documentation in the links to the left. To add a webhook manually, follow the steps below:

1.  Click on **Add webhook**
2.  Enter a name for the webhook (optional)
3.  Enter a URL for us to call whenever an event occurs
4.  Select which events you are interested in
5.  Click **Save**
6.  Done!

Whenever one of the events you selected occurs on your board, we will make a request to the URL you supplied with information about the event. If the request fails, the webhook will retry twice, waiting a short period of time in-between each attempt.  
  
**Note:** The callback URL must return status code 200 on an HTTP HEAD request. Otherwise it will not be added.
