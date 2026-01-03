workspace "Aideon Desktop" "Local‑first, time‑first EA platform" {
  !identifiers hierarchical

  model {
    user = person "EA Practitioner" "Explores portfolios and time‑sliced views"
    aideon = softwareSystem "Aideon Desktop" "Desktop app with Tauri host, React renderer, Rust worker crates" {
      renderer = container "Renderer (React in Tauri Webview)" "UI, no backend logic" "React"
      host = container "Host (Tauri/Rust)" "Typed IPC, window mgmt, security, worker orchestration" "Rust"
      worker = container "Worker (Chrona/Metis/Praxis)" "Time API + analytics behind traits" "Rust"
    }

    user -> renderer "Uses" "IPC bridge (typed)"
    renderer -> host "Invokes commands" "Tauri invoke (typed)"
    host -> worker "Calls engines via adapters" "in‑process (desktop), swappable adapters"
  }

  views {
    systemContext aideon "system-context" {
      include *
      autolayout lr
      title "Aideon Desktop — System Context"
    }

    container aideon "container" {
      include *
      autolayout lr
      title "Aideon Desktop — Container View"
    }

    styles {
      element "Person" {
        background #08427b
        color #ffffff
        shape Person
      }
      element "Software System" {
        background #1168bd
        color #ffffff
      }
      element "Container" {
        background #438dd5
        color #ffffff
      }
      relationship {
        routing Orthogonal
      }
    }
  }
}
