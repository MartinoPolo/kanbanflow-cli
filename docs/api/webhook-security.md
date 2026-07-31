# Security

> Source: https://kanbanflow.com/api-docs/webhook-security
> Scraped: 2026-07-30

# API Documentation

* * *

## Webhook signature verification

Each webhook has an associated **secret** you can use to verify that the request came from KanbanFlow. The request made to your URL endpoint will contain a special header called **X-KanbanFlow-Signature**. The signature is a HMAC-SHA256 hex digest of the payload using the secret as the key. Exactly how to verify the signature depends on your platform. Using Node.js for example, the signature can be verified as follows:

import crypto from 'crypto';  
  
function verifyKanbanFlowWebhookRequest(request, secret) {
  const content = request.body;
  const digest = crypto.createHmac('sha256', secret).update(content).digest('hex');
  const signature = request.headers\['x-kanbanflow-signature'\];
  return ('sha256=' + digest) === signature;
}
