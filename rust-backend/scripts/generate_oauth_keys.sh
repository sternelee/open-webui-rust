#!/bin/bash
# Generate Fernet encryption keys for OAuth configuration
# 
# This script generates secure Fernet keys compatible with the Rust backend's
# OAuth encryption system using Python's cryptography library.
#
# Usage:
#     ./generate_oauth_keys.sh

echo "========================================================================"
echo "OAuth Encryption Key Generator"
echo "========================================================================"
echo ""

# Check if Python 3 is available
if ! command -v python3 &> /dev/null; then
    echo "Error: python3 not found"
    echo "Please install Python 3 to generate Fernet keys"
    exit 1
fi

# Check if cryptography package is installed
if ! python3 -c "import cryptography" &> /dev/null; then
    echo "Error: cryptography package not installed"
    echo "Install it with: pip3 install cryptography"
    exit 1
fi

# Generate keys using Python
SESSION_KEY=$(python3 -c "from cryptography.fernet import Fernet; print(Fernet.generate_key().decode())")
CLIENT_KEY=$(python3 -c "from cryptography.fernet import Fernet; print(Fernet.generate_key().decode())")

echo "Generated secure Fernet keys for OAuth:"
echo ""
echo "----------------------------------------------------------------------"
echo "1. OAuth Session Token Encryption Key:"
echo "   (for encrypting OAuth tokens in the database)"
echo ""
echo "   OAUTH_SESSION_TOKEN_ENCRYPTION_KEY=$SESSION_KEY"
echo "----------------------------------------------------------------------"
echo ""
echo "----------------------------------------------------------------------"
echo "2. OAuth Client Info Encryption Key:"
echo "   (for encrypting MCP tool OAuth client information)"
echo ""
echo "   OAUTH_CLIENT_INFO_ENCRYPTION_KEY=$CLIENT_KEY"
echo "----------------------------------------------------------------------"
echo ""
echo "Add these to your .env file to enable OAuth functionality."
echo ""
echo "SECURITY NOTES:"
echo "   - Keep these keys SECRET and secure"
echo "   - Do NOT commit them to version control"
echo "   - Store them securely (e.g., in a password manager)"
echo "   - Use different keys for dev/staging/production"
echo "   - If keys are lost, all encrypted OAuth tokens will be unrecoverable"
echo ""
echo "========================================================================"
echo ""
echo "Quick copy-paste for your .env file:"
echo ""
echo "OAUTH_SESSION_TOKEN_ENCRYPTION_KEY=$SESSION_KEY"
echo "OAUTH_CLIENT_INFO_ENCRYPTION_KEY=$CLIENT_KEY"
echo ""

