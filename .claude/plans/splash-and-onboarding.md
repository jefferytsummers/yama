# Splash Page & Signup Funnel Plan

Web-first signup flow leading to desktop app download.

## Architecture

```
WEB (Public)                              DESKTOP APP (Authenticated)
─────────────                             ─────────────────────────────
┌──────────────┐                          ┌──────────────┐
│ Splash Page  │                          │  Dashboard   │
│  (Hero)      │                          │              │
└──────┬───────┘                          └──────┬───────┘
       ↓                                         ↓
┌──────────────┐                          ┌──────────────┐
│  Sign Up     │                          │ ProjectWizard│ ← Reused
│  Form        │                          │ (no signup)  │   component
└──────┬───────┘                          └──────────────┘
       ↓
┌──────────────┐
│ ProjectWizard│ ← Same component
│ + Signup ctx │
└──────┬───────┘
       ↓
┌──────────────┐
│ Success Page │
│ "Check email"│
│ Download CTA │
└──────────────┘
```

## Routes

| Route | Purpose | Auth |
|-------|---------|------|
| `/` | Splash/Hero page | Public |
| `/signup` | Signup form | Public |
| `/onboarding` | Project wizard (post-signup) | Requires signup |
| `/onboarding/success` | Download instructions | Requires signup |
| `/projects` | Dashboard (app only) | Auth required |
| `/projects/new` | Project wizard (in-app) | Auth required |

---

## Phase 1: Splash Page (Public Landing)

### Purpose
Marketing page that converts visitors to signups.

### Layout
```
┌────────────────────────────────────────────────────────────────┐
│  [Logo] YAMA                          [Log In]  [Get Started]  │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│                           △                                    │
│                          ╱ ╲                                   │
│                         ╱   ╲                                  │
│                                                                │
│            Video Intelligence for Professionals                │
│                                                                │
│    Frame-accurate analysis • Real-time AI inference            │
│    Local-first privacy • Works offline                         │
│                                                                │
│              [★ Start Free]    [Watch Demo →]                  │
│                                                                │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                   [Product Screenshot]                   │   │
│  │                   or Demo Video                          │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                │
├────────────────────────────────────────────────────────────────┤
│                        How It Works                            │
│                                                                │
│  ┌──────────┐     ┌──────────┐     ┌──────────┐               │
│  │ 1. Import│     │ 2. Analyze│    │ 3. Ask   │               │
│  │  Videos  │ →   │  Content │ →   │Questions │               │
│  └──────────┘     └──────────┘     └──────────┘               │
│                                                                │
├────────────────────────────────────────────────────────────────┤
│                        Features                                │
│                                                                │
│  🎯 Find Moments        🔒 Privacy First    ⚡ Real-time       │
│  Search by describing   All processing      Stream analysis    │
│  what you're looking    happens locally     as you watch       │
│  for in plain English   on your machine                        │
│                                                                │
├────────────────────────────────────────────────────────────────┤
│                      Ready to Start?                           │
│                                                                │
│                    [Create Free Account]                       │
│                                                                │
│              No credit card • Free forever tier                │
│                                                                │
├────────────────────────────────────────────────────────────────┤
│  © 2024 Yama   •   Privacy   •   Terms   •   Docs              │
└────────────────────────────────────────────────────────────────┘
```

### Components Needed
- `SplashHero` - Main hero section
- `FeatureCard` - Feature highlight cards
- `HowItWorks` - Step-by-step visual
- `Footer` - Links and legal

---

## Phase 2: Signup Flow

### Signup Form (`/signup`)
```
┌────────────────────────────────────────┐
│                                        │
│             Create Account             │
│                                        │
│  ┌──────────────────────────────────┐  │
│  │ Email                            │  │
│  └──────────────────────────────────┘  │
│                                        │
│  ┌──────────────────────────────────┐  │
│  │ Password                         │  │
│  └──────────────────────────────────┘  │
│                                        │
│  ┌──────────────────────────────────┐  │
│  │ Confirm Password                 │  │
│  └──────────────────────────────────┘  │
│                                        │
│         [Create Account]               │
│                                        │
│  ─────────── or ───────────            │
│                                        │
│  [G] Continue with Google              │
│  [] Continue with GitHub               │
│                                        │
│  Already have an account? [Log in]     │
│                                        │
└────────────────────────────────────────┘
```

### Post-Signup Redirect
After successful signup → `/onboarding` (Project Wizard)

---

## Phase 3: Project Wizard (Reusable)

### Component: `ProjectWizard`

**Props:**
```typescript
interface ProjectWizardProps {
  context: 'onboarding' | 'app';  // Determines final step
  onComplete: (project: ProjectData) => void;
  onCancel?: () => void;
}
```

**Behavior by context:**

| Aspect | `onboarding` (web signup) | `app` (in-app) |
|--------|---------------------------|----------------|
| Final step | "Check your email!" | "Project created!" |
| Cancel | Goes to splash | Closes modal |
| On complete | Redirect to success | Add to project list |
| Header | Full page | Modal |

### Steps (shared)

1. **Details** - Name, description, tags
2. **Videos** - Drop zone (optional in onboarding, shows "import later")
3. **Models** - Preset selection
4. **Tools** - Tool toggles
5. **Review** - Summary + Create

### Context-Specific Final Step

**Onboarding context:**
```
┌────────────────────────────────────────────────────────────────┐
│                                                                │
│                          ✓                                     │
│                                                                │
│                 You're all set!                                │
│                                                                │
│    We've sent setup instructions to                            │
│    you@example.com                                             │
│                                                                │
│    ┌────────────────────────────────┐                          │
│    │   Download Yama for macOS      │                          │
│    │   [↓ Download .dmg]            │                          │
│    └────────────────────────────────┘                          │
│                                                                │
│    Also available for:                                         │
│    Windows • Linux • iOS • Android                             │
│                                                                │
│    Your project "Q4 Planning" will be ready                    │
│    when you open the app.                                      │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

**App context:**
```
┌────────────────────────────────────────────────────────────────┐
│                                                                │
│                          ✓                                     │
│                                                                │
│              Project created!                                  │
│                                                                │
│    "Q4 Planning" is ready.                                     │
│                                                                │
│    [Open Project]    [Create Another]                          │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

---

## Phase 4: Success Page (`/onboarding/success`)

Standalone page shown after wizard completion in onboarding context.

- Download links for all platforms
- Email confirmation reminder
- "Open in app" deep link (if app installed)

---

## Implementation Plan

### Week 1: Foundation

**Day 1-2: Splash Page**
- [ ] `SplashHero.svelte` - Hero section with CTA
- [ ] `FeatureCard.svelte` - Feature highlights
- [ ] `HowItWorks.svelte` - Process visualization
- [ ] `SplashFooter.svelte` - Footer links
- [ ] Update `routes/+page.svelte` - Assemble splash

**Day 2-3: Auth Components**
- [ ] `SignupForm.svelte` - Email/password form
- [ ] `OAuthButtons.svelte` - Google/GitHub buttons
- [ ] `routes/signup/+page.svelte` - Signup page
- [ ] `routes/login/+page.svelte` - Login page (stub)

### Week 2: Wizard

**Day 4-5: Wizard Infrastructure**
- [ ] `StepIndicator.svelte` - Progress stepper
- [ ] `TextArea.svelte` - Multi-line input
- [ ] `TagInput.svelte` - Tag chips
- [ ] `ToolToggle.svelte` - Tool switches
- [ ] `PresetCard.svelte` - Preset selection
- [ ] `ModelChip.svelte` - Model display

**Day 6-7: ProjectWizard Assembly**
- [ ] `ProjectWizard.svelte` - Main wizard container
- [ ] Wizard step components (5 steps)
- [ ] Context-aware final step
- [ ] `routes/onboarding/+page.svelte`
- [ ] `routes/onboarding/success/+page.svelte`

### Week 3: Integration

**Day 8: App Integration**
- [ ] `routes/projects/+page.svelte` - Dashboard stub
- [ ] `routes/projects/new/+page.svelte` - In-app wizard
- [ ] Wizard modal mode for app context

**Day 9: Polish**
- [ ] Animations and transitions
- [ ] Form validation
- [ ] Error states
- [ ] Loading states
- [ ] Mobile responsiveness

---

## Data Types

```typescript
// lib/types/auth.ts
interface User {
  id: string;
  email: string;
  name?: string;
  createdAt: string;
}

interface SignupData {
  email: string;
  password: string;
}

// lib/types/wizard.ts
interface ProjectData {
  name: string;
  description: string;
  tags: string[];
  files: UploadedFile[];
  selectedPreset: PresetId;
  enabledTools: ToolId[];
}

type WizardContext = 'onboarding' | 'app';

interface WizardStep {
  id: string;
  label: string;
  isComplete: (data: ProjectData) => boolean;
}
```

---

## Files to Create

### Components
| File | Purpose |
|------|---------|
| `lib/components/SplashHero.svelte` | Hero section |
| `lib/components/FeatureCard.svelte` | Feature cards |
| `lib/components/HowItWorks.svelte` | Process steps |
| `lib/components/SplashFooter.svelte` | Footer |
| `lib/components/SignupForm.svelte` | Auth form |
| `lib/components/OAuthButtons.svelte` | Social login |
| `lib/components/StepIndicator.svelte` | Wizard progress |
| `lib/components/TextArea.svelte` | Multi-line input |
| `lib/components/TagInput.svelte` | Tag chips |
| `lib/components/ToolToggle.svelte` | Tool switches |
| `lib/components/PresetCard.svelte` | Preset selection |
| `lib/components/ModelChip.svelte` | Model info |
| `lib/components/ProjectWizard.svelte` | Main wizard |
| `lib/components/WizardSuccess.svelte` | Success states |

### Routes
| File | Purpose |
|------|---------|
| `routes/+page.svelte` | Splash page (update) |
| `routes/signup/+page.svelte` | Signup form |
| `routes/login/+page.svelte` | Login form |
| `routes/onboarding/+page.svelte` | Post-signup wizard |
| `routes/onboarding/success/+page.svelte` | Download page |
| `routes/projects/+page.svelte` | Dashboard (stub) |
| `routes/projects/new/+page.svelte` | In-app wizard |

### Types
| File | Purpose |
|------|---------|
| `lib/types/auth.ts` | Auth types |
| `lib/types/wizard.ts` | Wizard types |

---

## Mock Data (No Backend Yet)

```typescript
// Simulated auth state
let isAuthenticated = false;
let currentUser: User | null = null;

// Simulated signup
function mockSignup(data: SignupData): Promise<User> {
  return new Promise((resolve) => {
    setTimeout(() => {
      const user = {
        id: crypto.randomUUID(),
        email: data.email,
        createdAt: new Date().toISOString(),
      };
      isAuthenticated = true;
      currentUser = user;
      resolve(user);
    }, 1000);
  });
}
```

---

## Acceptance Criteria

### Splash Page
- [ ] Hero section with clear value proposition
- [ ] "Start Free" CTA prominent
- [ ] Feature cards highlight key benefits
- [ ] How it works section explains process
- [ ] Footer with links
- [ ] Mobile responsive

### Signup
- [ ] Email/password form with validation
- [ ] Password confirmation match
- [ ] OAuth buttons (visual only, not wired)
- [ ] Link to login page
- [ ] Success redirects to onboarding

### Project Wizard
- [ ] 5 steps with progress indicator
- [ ] Back/Next navigation
- [ ] Validation per step
- [ ] Context-aware final step
- [ ] Works in both onboarding and app contexts

### Success Page
- [ ] Shows user's email
- [ ] Download button for macOS
- [ ] Platform alternatives listed
- [ ] Project name mentioned

---

## Future Considerations

- **Backend API**: Actual auth endpoints (Phase 4+)
- **Email verification**: Real email sending
- **OAuth**: Google/GitHub integration
- **Deep linking**: `yama://` protocol for app handoff
- **Project sync**: Cloud sync between web config and app
