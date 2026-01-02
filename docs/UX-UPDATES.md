# Aideon Desktop UX Modernization Plan

This document outlines the strategy to modernize the User Experience (UX) and User Interface (UI) of
the Aideon Desktop application. The goal is to establish a professional, cohesive, and scalable
design system that aligns with the Aideon Desktop shell contract in `app/AideonDesktop/DESIGN.md`
and provides a respectful, native-like experience across macOS, Windows, and Linux.

## Core Philosophy: "Native-Tier Corporate"

We are moving away from "Playful Startup" (round corners, quirky fonts) to "Professional Tool" (precise, clean, system-integrated).

## 1. Design Tokens & Global Theming (Refined)

**Objective:** Establish a modern, consistent visual foundation that feels native.

- **Typography (Crucial for OS Respect):**
  - **Issue:** Current `brand` token prioritizes `'Space Grotesk'`. This is a display font and reduces legibility/professionalism in dense UI (data tables, trees).
  - **Action:**
    - **Headings:** Keep `Space Grotesk` ONLY for high-level headers (H1/H2) or Marketing surfaces.
    - **UI/Body:** Prioritize System Fonts (`-apple-system`, `Segoe UI`, `Roboto`) or `Inter` for a neutral corporate look.
    - **Code:** Ensure `JetBrains Mono` or `SF Mono` is available.
- **Radius:**
  - **Standard:** `0.5rem` (8px). This strikes the balance between modern (soft) and professional (crisp).
  - **Inputs/Buttons:** Must strictly inherit `var(--radius)`.
- **Colors & Contrast:**
  - **Backgrounds:** Avoid gradients in the shell. Use solid `bg-background`.
  - **Separators:** Use `border-border` (approx gray-200/gray-800).
  - **Text:** High contrast text `text-foreground` for legibility. Muted text `text-muted-foreground` for metadata.

## 2. Shadcn/UI Adherence Strategy

**Objective:** Ensure we are _extending_ the library, not fighting it.

- **Rule of Law:**
  - **No Magic Numbers:** CSS classes like `w-[245px]` are forbidden unless representing a user-resizable value. Use `w-64`, `w-72`, or layout-based sizing.
  - **Variant Usage:** Use `cva` (Class Variance Authority) for all component states. Do not manually toggle classes like `active ? 'bg-blue-500' : ''`. Instead, define an intent variant (e.g., `variant="active"` or `data-state="active"`).
  - **Slot Pattern:** Utilize Radix UI `asChild` / `Slot` pattern to compose behavior (e.g., a Sidebar Item that acts as a Link).

## 3. Operating System Respect (Cross-Platform Polish)

**Objective:** The app should not look like a "website in a box".

- **Window Controls:**
  - **macOS:** Ensure the top-left toolbar area (`h-12` approx) is clear of interactive elements to allow for the window drag region and traffic lights.
  - **Windows:** Ensure window control buttons (Minimize, Maximize, Close) are rendered or the frame allows the system ones.
- **Scrollbars:**
  - **Action:** Add global CSS to style scrollbars.
    - **Mac:** Native auto-hiding is fine.
    - **Windows:** Webkit scrollbars must be styled to be thin and neutral (`bg-transparent`, thumb `bg-muted-foreground/30` hover `bg-muted-foreground/50`). Default Windows scrollbars are too wide and jarring.
- **Focus States:**
  - Use `focus-visible` ring. Do not remove outlines without replacing them with a high-visibility alternative.

## 4. Canvas & Widget Modernization

**Objective:** Transform the canvas into a dynamic, infinite workspace that supports professional diagramming and data visualization.

- **Infinite Canvas Background:**
  - **Visuals:** Move from `bg-card` to a "Dot Grid" or subtle "Mesh" pattern (`bg-dot-black/[0.2]` or similar utility). This cues the user that the space is infinite.
  - **Layout:** The current CSS Grid (`AideonCanvasRuntime`) restricts the "infinite" feel.
  - **Action:**
    - Update `AideonCanvasRuntime` to support a layout mode where widgets can be freely positioned or, if keeping the grid, make the grid itself feel like a surface _on top_ of an infinite void.
    - For now, we will modernize the **Grid View** to look like a dashboard, but prepare the styling for infinite pan/zoom.
- **Widget Containers:**
  - **Style:** Replace simple `div` borders with a **Glassmorphic** or **Solid Card** container that floats above the background.
  - **Header:** Add a consistent "Widget Chrome" (Title, Drag Handle, Actions Menu) to every widget wrapper.
  - **Shadows:** Use `shadow-md` for standard widgets, `shadow-xl` when dragging or active.
- **Page Boundary Awareness:**
  - **Objective:** Allow users to see where their content lands for export (PDF/A4).
  - **Action:** Add a "Page View" toggle. When enabled, render a dashed overlay (`border-dashed border-primary/20`) representing A4/Letter pages on the background.

## 5. Execution Checklist

0.  **Shell alignment:** Ensure all workspaces render inside `AideonDesktopShell` slots (navigation,
    toolbar, content, inspector) per `app/AideonDesktop/DESIGN.md`.
1.  **Typography Reform:** Update `tokens.ts` and `styles.css` to demote Space Grotesk.
2.  **OS Scrollbars:** Inject webkit scrollbar styles in `globals.css`.
3.  **Sidebar Polish:** Refactor `projects-sidebar.tsx` to strictly use Shadcn sidebar primitives and tokens.
4.  **Toolbar/Window Drag:** Verify the header allows for window dragging.
5.  **Component Audit:** Review `Button`, `Input`, `Card` for radius consistency.
6.  **Canvas Modernization:**
    - Update `AideonCanvasRuntime` with a "Dot Grid" background.
    - Create a `WidgetFrame` component (Title, Actions, Container) to wrap all widgets.
    - Implement "Page Break" visualizer (CSS-based grid overlay).
