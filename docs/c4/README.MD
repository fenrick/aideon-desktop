## C4 Diagrams (DSL Source)

This folder contains the Structurizr DSL source for the Aideon Desktop System Context and Container
views used during M0. CI no longer renders diagrams; these are design-time artifacts.

### Local rendering (optional)

Requirements: Java 17+, Graphviz (`dot`).

Option A — Use Structurizr CLI + PlantUML directly

1. Download the latest Structurizr CLI ZIP (temporary):
   - macOS/Linux:
     - `TMP_ZIP=$(mktemp -t structurizr-cli.XXXXXX.zip)`
     - `curl -fSL -o "$TMP_ZIP" https://github.com/structurizr/cli/releases/latest/download/structurizr-cli.zip`
     - `mkdir -p .tools/structurizr-cli && unzip -q -o "$TMP_ZIP" -d .tools/structurizr-cli && rm -f "$TMP_ZIP"`
   - Windows (PowerShell):
     - `$tmp = New-TemporaryFile; Invoke-WebRequest -OutFile $tmp https://github.com/structurizr/cli/releases/latest/download/structurizr-cli.zip; Expand-Archive -Force $tmp .tools/structurizr-cli; Remove-Item $tmp`
2. Download PlantUML JAR (or use your system package):
   - `mkdir -p .tools/plantuml && curl -fSL -o .tools/plantuml/plantuml.jar https://github.com/plantuml/plantuml/releases/latest/download/plantuml.jar`
3. Export PlantUML and render PNGs:
   - `mkdir -p docs/c4/out/plantuml docs/c4/out/png`
   - `java -jar .tools/structurizr-cli/structurizr-cli-*.jar export -workspace docs/c4/workspace.dsl -format plantuml -output docs/c4/out/plantuml`
   - `java -Djava.awt.headless=true -jar .tools/plantuml/plantuml.jar -tpng docs/c4/out/plantuml/*.puml -o docs/c4/out/png`

Option B — Use Docker

- `docker run --rm -v "$PWD":/work ghcr.io/structurizr/cli:latest \
export -workspace /work/docs/c4/workspace.dsl -format plantuml -output /work/docs/c4/out/plantuml`
- Then render PNGs via PlantUML container or local tools.

Outputs are written under `docs/c4/out/plantuml` and `docs/c4/out/png` (gitignored).

### Files

- `workspace.dsl` – DSL source (System Context + Container views)
- `out/` – CI-generated exports (not committed); available as workflow artifacts
