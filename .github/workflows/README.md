# GitHub Workflows

## RPC Tests
This workflow runs when changes are pushed to the `main` branch or on pull requests targeting `main`. It builds the project, runs tests, clippy, and checks formatting.

## Buf Push
This workflow pushes your Protocol Buffer definitions to [buf.build](https://buf.build). It runs in the following scenarios:
- When changes are pushed to the `main` branch and affect proto files or the buf.yaml config
- After the "RPC Tests" workflow completes successfully on the `main` branch
- Manually via workflow dispatch

### Setup Required
To use the Buf Push workflow, you need to set up a secret in your GitHub repository:

1. Create a Buf API token:
   - Go to [buf.build](https://buf.build) and sign in
   - Navigate to your user settings or organization settings
   - Go to "API Keys" and generate a new token with write access

2. Add the token to GitHub secrets:
   - Go to your GitHub repository
   - Navigate to Settings > Secrets and variables > Actions
   - Click "New repository secret"
   - Name: `BUF_TOKEN`
   - Value: Your Buf API token
   - Click "Add secret"

The workflow will automatically use this token to authenticate with the Buf Schema Registry. 