# 🛡️ Dracon Security (The Warden)

The Warden is a high-performance, standalone security program that provides transparent, in-place encryption for secrets stored in your Git repositories.

## 🚀 Getting Started

### 1. Identity Setup
Create your local machine identity:
```bash
dracon-security init-identity
```

### 2. Global Installation
```bash
dracon-security install-git-filter --global
```

## 🛠️ CLI Reference

| Command | Purpose |
| :--- | :--- |
| `init-identity` | Generates your master key ring. |
| `show-identity` | Lists all active keys on this machine. |
| `authorize-project-key` | Grant access to another machine. |
| `encrypt-in-situ` | Manually scan and encrypt a text file. |
| `decrypt-in-situ` | Reverses encryption tags back to plaintext. |
| `guide` | Show the built-in walkthrough. |
