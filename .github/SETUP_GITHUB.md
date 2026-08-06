# GitHub Repository Setup Instructions

Follow these steps to properly configure the Himalayas Browser GitHub repository.

## 1. Go to Repository Settings

Visit: https://github.com/Mullassery/Himalayas-Browser/settings

## 2. Update Repository Details (About Section)

### Description (Required - max 125 characters)
```
The world's first truly agent-native browser platform. Agents are native citizens.
```

### Website URL (Optional)
```
https://github.com/Mullassery/Himalayas-Browser
```
Or set to your custom domain when available.

## 3. Add Topics

Go to **Settings** → **General** → **Repository topics**

Add these topics (click each to add):
- `agent-browser`
- `agent-native`
- `rust`
- `headless-browser`
- `automation`
- `privacy-first`
- `browser-platform`
- `keyboard-driven`
- `multi-platform`
- `ai-native`

## 4. Enable Features

Go to **Settings** → **General** → **Features**

✅ Enable:
- [x] Discussions
- [x] Projects (optional)
- [x] Wiki (for documentation links)

Disable:
- [ ] Packages (not using)
- [ ] Deployments (future)

## 5. Set Default Branch

Go to **Settings** → **General** → **Default branch**

Ensure it's set to: `main`

## 6. Social Preview

Go to **Settings** → **General** → **Social preview**

Upload a banner image (optional - GitHub will use README preview if not set):
- Dimensions: 1280×640px
- Format: PNG or JPG
- Suggested: Screenshot of CLI or logo

## 7. Collaborators & Permissions

Go to **Settings** → **Collaborators and teams**

Add team members as needed:
- Maintainers: Write access
- Contributors: Write access for approved PRs
- Community: Read access (public)

## 8. Branch Protection Rules

Go to **Settings** → **Branches** → **Add rule**

For `main` branch:
- [x] Require a pull request before merging
- [x] Require status checks to pass (CI/CD)
- [x] Require branches to be up to date
- [x] Dismiss stale PR approvals
- [x] Require code review from code owners (1+ review)

## 9. Actions Permissions

Go to **Settings** → **Actions** → **General**

- Allow all actions and reusable workflows (if not already)
- Fork pull request workflows: Read repository contents permission

## 10. GitHub Pages (Optional)

If you want to host documentation:

Go to **Settings** → **Pages**

- Source: Deploy from a branch
- Branch: `main` (or create `gh-pages`)
- Folder: `/docs` (where our markdown files are)

## Result

Your GitHub repository will now display:
- ✅ Clear description
- ✅ Relevant topics
- ✅ Links to documentation
- ✅ Professional appearance
- ✅ Community engagement features

## Verification

After setup, your repository should show:
1. Description: "The world's first truly agent-native browser platform..."
2. Topics: agent-browser, rust, headless-browser, etc.
3. README prominently displayed
4. Quick access to docs/
5. Professional About section

---

**Note**: Some settings may require GitHub admin access or organizational permissions.
For questions, contact: mullassery@gmail.com
