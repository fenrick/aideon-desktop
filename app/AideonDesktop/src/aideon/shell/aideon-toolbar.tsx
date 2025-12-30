import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  useSyncExternalStore,
  type ComponentPropsWithoutRef,
  type ReactNode,
} from 'react';

import { Menubar, MenubarContent, MenubarItem, MenubarMenu, MenubarTrigger } from 'design-system';
import { Toolbar, ToolbarSection, ToolbarSeparator } from 'design-system/blocks/toolbar';
import { Badge } from 'design-system/components/ui/badge';
import { Button } from 'design-system/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from 'design-system/components/ui/dropdown-menu';
import { Kbd } from 'design-system/components/ui/kbd';
import { cn } from 'design-system/lib/utilities';
import { isDevelopmentBuild, isTauriRuntime } from 'lib/runtime';
import {
  CommandIcon,
  LaptopIcon,
  MoonIcon,
  PanelRightClose,
  PanelRightOpen,
  SunIcon,
} from 'lucide-react';
import { useTheme } from 'next-themes';
import { useAideonShellControls } from './shell-controls';

import { SidebarTrigger, useSidebar } from 'design-system/desktop-shell';
import { AideonCommandPalette, type AideonCommandItem } from './command-palette';
import { KeyboardShortcutsDialog } from './keyboard-shortcuts-dialog';

export interface AideonToolbarProperties extends Readonly<ComponentPropsWithoutRef<'div'>> {
  readonly title: string;
  readonly subtitle?: string;
  readonly modeLabel?: string;
  readonly start?: ReactNode;
  readonly center?: ReactNode;
  readonly workspaceToolbar?: ReactNode;
  readonly end?: ReactNode;
  readonly statusMessage?: string;
  readonly commands?: readonly AideonCommandItem[];
  readonly onShellCommand?: (command: string, payload?: unknown) => void;
}

/**
 * Safely read sidebar controls when the toolbar is rendered outside a SidebarProvider (tests).
 */
function useOptionalSidebar() {
  try {
    return useSidebar();
  } catch {
    return;
  }
}

/**
 * Best-effort platform detection without deprecated `navigator.platform`.
 */
function isMacPlatform(): boolean {
  if (typeof navigator === 'undefined') {
    return false;
  }
  const ua = navigator.userAgent.toLowerCase();
  return ua.includes('mac') || ua.includes('iphone') || ua.includes('ipad') || ua.includes('ipod');
}

/**
 * No-op cleanup function for `useSyncExternalStore` subscriptions.
 */
function noop() {
  void 0;
}

/**
 * No-op subscription for `useSyncExternalStore`.
 * @returns Unsubscribe callback.
 */
function noopSubscribe() {
  return noop;
}

/**
 * Client snapshot accessor for macOS detection.
 * @returns True when the user agent is macOS.
 */
function getIsMacSnapshot() {
  return isMacPlatform();
}

/**
 * Server snapshot accessor for macOS detection.
 * @returns Always false during SSR.
 */
function getServerIsMacSnapshot() {
  return false;
}

/**
 * Check whether an event target is a text-editable element.
 * @param target - Event target from a keydown listener.
 * @returns True when the target should receive text input.
 */
function isEditableTarget(target: EventTarget | null): boolean {
  if (!target || !(target instanceof HTMLElement)) {
    return false;
  }
  const tag = target.tagName.toLowerCase();
  return tag === 'input' || tag === 'textarea' || target.isContentEditable;
}

/**
 * Stable platform detection for SSR/CSR with no hydration mismatch.
 * @returns True when running on macOS.
 */
function useIsMacPlatform(): boolean {
  return useSyncExternalStore(noopSubscribe, getIsMacSnapshot, getServerIsMacSnapshot);
}

/**
 * Safely read the theme context when present.
 */
function useOptionalTheme() {
  try {
    return useTheme();
  } catch {
    return;
  }
}

/**
 * Build the command palette list from shell + workspace command sources.
 * @param root0 - Inputs for command construction.
 * @param root0.sidebar - Sidebar controls when available.
 * @param root0.shell - Shell controls when available.
 * @param root0.theme - Theme controls when available.
 * @param root0.workspaceCommands - Workspace-provided commands.
 * @param root0.shortcutLabelFor - Formatter for shortcut labels.
 * @returns Palette-ready command items.
 */
function buildShellCommands({
  sidebar,
  shell,
  theme,
  workspaceCommands,
  shortcutLabelFor,
}: {
  readonly sidebar: ReturnType<typeof useOptionalSidebar>;
  readonly shell: ReturnType<typeof useAideonShellControls>;
  readonly theme: ReturnType<typeof useOptionalTheme>;
  readonly workspaceCommands: readonly AideonCommandItem[];
  readonly shortcutLabelFor: (letter: string) => string;
}): AideonCommandItem[] {
  const viewCommands: AideonCommandItem[] = [
    ...(sidebar
      ? ([
          {
            id: 'toggle-navigation',
            group: 'View',
            label: 'Toggle navigation',
            shortcut: shortcutLabelFor('B'),
            onSelect: () => {
              sidebar.toggleSidebar();
            },
          },
        ] satisfies AideonCommandItem[])
      : []),
    ...(shell
      ? ([
          {
            id: 'toggle-inspector',
            group: 'View',
            label: 'Toggle inspector',
            shortcut: shortcutLabelFor('I'),
            onSelect: () => {
              shell.toggleInspector();
            },
          },
        ] satisfies AideonCommandItem[])
      : []),
  ];

  const themeCommands: AideonCommandItem[] = theme
    ? ([
        {
          id: 'theme.system',
          group: 'Theme',
          label: 'Use system theme',
          onSelect: () => {
            theme.setTheme('system');
          },
        },
        {
          id: 'theme.light',
          group: 'Theme',
          label: 'Light theme',
          onSelect: () => {
            theme.setTheme('light');
          },
        },
        {
          id: 'theme.dark',
          group: 'Theme',
          label: 'Dark theme',
          onSelect: () => {
            theme.setTheme('dark');
          },
        },
      ] satisfies AideonCommandItem[])
    : [];

  return [...viewCommands, ...themeCommands, ...workspaceCommands];
}

/**
 *
 * @param root0
 * @param root0.isTauri
 * @param root0.sidebar
 * @param root0.shell
 * @param root0.openCommandPalette
 */
function useBrowserShortcutHandler({
  isTauri,
  sidebar,
  shell,
  openCommandPalette,
}: {
  readonly isTauri: boolean;
  readonly sidebar: ReturnType<typeof useOptionalSidebar>;
  readonly shell: ReturnType<typeof useAideonShellControls>;
  readonly openCommandPalette: () => void;
}) {
  useEffect(() => {
    if (isTauri) {
      return;
    }
    const handleKeydown = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey)) {
        return;
      }
      if (isEditableTarget(event.target)) {
        return;
      }
      const key = event.key.toLowerCase();
      const didHandle = handleBrowserShortcut({
        key,
        sidebar,
        shell,
        openCommandPalette,
      });
      if (didHandle) {
        event.preventDefault();
      }
    };
    globalThis.addEventListener('keydown', handleKeydown);
    return () => {
      globalThis.removeEventListener('keydown', handleKeydown);
    };
  }, [isTauri, openCommandPalette, shell, sidebar]);
}

/**
 *
 * @param root0
 * @param root0.isTauri
 * @param root0.sidebar
 * @param root0.shell
 * @param root0.onShellCommand
 * @param root0.openCommandPalette
 */
function useTauriShellCommandListener({
  isTauri,
  sidebar,
  shell,
  onShellCommand,
  openCommandPalette,
}: {
  readonly isTauri: boolean;
  readonly sidebar: ReturnType<typeof useOptionalSidebar>;
  readonly shell: ReturnType<typeof useAideonShellControls>;
  readonly onShellCommand?: (command: string, payload?: unknown) => void;
  readonly openCommandPalette: () => void;
}) {
  useEffect(() => {
    if (!isTauri) {
      return;
    }
    let cancelled = false;
    let unlisten: undefined | (() => void);

    const subscribe = async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        if (cancelled) {
          return;
        }
        unlisten = await listen<{ command?: string; payload?: unknown }>(
          'aideon.shell.command',
          (event) => {
            const command = event.payload.command;
            const payload = event.payload.payload;

            if (command === 'toggle-navigation') {
              sidebar?.toggleSidebar();
            }
            if (command === 'toggle-inspector') {
              shell?.toggleInspector();
            }
            if (command === 'open-command-palette') {
              openCommandPalette();
            }
            if (command === 'file.print') {
              globalThis.print();
            }

            if (command) {
              onShellCommand?.(command, payload);
            }
          },
        );
      } catch {
        // ignore missing tauri event module (browser preview)
      }
    };

    subscribe().catch((_ignoredError: unknown) => {
      // ignore missing tauri event module (browser preview)
    });

    return () => {
      cancelled = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, [isTauri, onShellCommand, openCommandPalette, shell, sidebar]);
}

/**
 * Handle Cmd/Ctrl key combos in browser preview mode.
 * @param root0 - Shortcut context.
 * @param root0.key - Pressed key (lowercase).
 * @param root0.sidebar - Sidebar controls when available.
 * @param root0.shell - Shell controls when available.
 * @param root0.openCommandPalette - Opens the command palette.
 * @returns True when handled.
 */
function handleBrowserShortcut({
  key,
  sidebar,
  shell,
  openCommandPalette,
}: {
  readonly key: string;
  readonly sidebar: ReturnType<typeof useOptionalSidebar>;
  readonly shell: ReturnType<typeof useAideonShellControls>;
  readonly openCommandPalette: () => void;
}): boolean {
  switch (key) {
    case 'b': {
      if (!sidebar) {
        return false;
      }
      sidebar.toggleSidebar();
      return true;
    }
    case 'i': {
      if (!shell) {
        return false;
      }
      shell.toggleInspector();
      return true;
    }
    case 'k': {
      openCommandPalette();
      return true;
    }
    default: {
      return false;
    }
  }
}

/**
 *
 * @param root0
 * @param root0.theme
 */
function ThemeMenu({
  theme,
}: {
  readonly theme: NonNullable<ReturnType<typeof useOptionalTheme>>;
}) {
  const currentTheme = theme.theme ?? 'system';
  const icon = (() => {
    if (currentTheme === 'dark') {
      return <MoonIcon className="size-4" />;
    }
    if (currentTheme === 'light') {
      return <SunIcon className="size-4" />;
    }
    return <LaptopIcon className="size-4" />;
  })();

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button type="button" variant="ghost" size="icon" className="size-7" aria-label="Theme">
          {icon}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem
          onSelect={() => {
            theme.setTheme('system');
          }}
        >
          System
        </DropdownMenuItem>
        <DropdownMenuItem
          onSelect={() => {
            theme.setTheme('light');
          }}
        >
          Light
        </DropdownMenuItem>
        <DropdownMenuItem
          onSelect={() => {
            theme.setTheme('dark');
          }}
        >
          Dark
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/**
 *
 * @param root0
 * @param root0.sidebar
 * @param root0.shell
 * @param root0.isTauri
 * @param root0.isDevelopment
 * @param root0.openStyleguide
 * @param root0.shortcutLabelFor
 * @param root0.start
 * @param root0.title
 * @param root0.subtitle
 * @param root0.modeLabel
 * @param root0.onOpenCommandPalette
 * @param root0.onOpenShortcuts
 */
function ToolbarStartSection({
  sidebar,
  shell,
  isTauri,
  isDevelopment,
  openStyleguide,
  shortcutLabelFor,
  start,
  title,
  subtitle,
  modeLabel,
  onOpenCommandPalette,
  onOpenShortcuts,
}: {
  readonly sidebar: ReturnType<typeof useOptionalSidebar>;
  readonly shell: ReturnType<typeof useAideonShellControls>;
  readonly isTauri: boolean;
  readonly isDevelopment: boolean;
  readonly openStyleguide: () => void;
  readonly shortcutLabelFor: (letter: string) => string;
  readonly start?: ReactNode;
  readonly title: string;
  readonly subtitle?: string;
  readonly modeLabel?: string;
  readonly onOpenCommandPalette: () => void;
  readonly onOpenShortcuts: () => void;
}) {
  return (
    <ToolbarSection className="min-w-0 gap-2">
      {sidebar ? <SidebarTrigger className="size-7" /> : undefined}
      {shell ? (
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="size-7"
          aria-label="Toggle inspector"
          onClick={() => {
            shell.toggleInspector();
          }}
        >
          {shell.inspectorCollapsed ? (
            <PanelRightOpen className="size-4" />
          ) : (
            <PanelRightClose className="size-4" />
          )}
        </Button>
      ) : undefined}
      {isTauri ? undefined : (
        <AppMenu
          onOpenCommandPalette={onOpenCommandPalette}
          onOpenShortcuts={onOpenShortcuts}
          onOpenStyleguide={openStyleguide}
          shortcutLabelFor={shortcutLabelFor}
          showDebugItems={isDevelopment}
        />
      )}
      {start}
      <Button
        type="button"
        variant="outline"
        size="sm"
        className="hidden h-8 gap-2 md:inline-flex"
        aria-label="Open command palette"
        onClick={onOpenCommandPalette}
      >
        <CommandIcon className="size-4" />
        Commands
        <Kbd className="ml-1">{shortcutLabelFor('K')}</Kbd>
      </Button>
      <ToolbarSeparator />
      <div className="min-w-0">
        <div className="flex items-center gap-2">
          <span className="truncate text-sm font-semibold tracking-tight">{title}</span>
          {modeLabel ? (
            <Badge variant="secondary" className="hidden sm:inline-flex">
              {modeLabel}
            </Badge>
          ) : undefined}
        </div>
        {subtitle ? (
          <div className="truncate text-xs text-muted-foreground">{subtitle}</div>
        ) : undefined}
      </div>
    </ToolbarSection>
  );
}

/**
 *
 * @param root0
 * @param root0.workspaceToolbar
 * @param root0.center
 */
function ToolbarCenterSection({ center }: { readonly center?: ReactNode }) {
  return (
    <ToolbarSection justify="center" className="hidden min-w-0 max-w-[520px] px-2 md:flex">
      {center}
    </ToolbarSection>
  );
}

/**
 *
 * @param root0
 * @param root0.end
 * @param root0.theme
 */
function ToolbarEndSection({
  end,
  theme,
}: {
  readonly end?: ReactNode;
  readonly theme: ReturnType<typeof useOptionalTheme>;
}) {
  return (
    <ToolbarSection justify="end" className="gap-2">
      {end}
      {theme ? <ThemeMenu theme={theme} /> : undefined}
    </ToolbarSection>
  );
}

/**
 *
 * @param root0
 * @param root0.statusMessage
 */
function ToolbarStatusMessage({ statusMessage }: { readonly statusMessage?: string }) {
  return (
    <div
      className={cn(
        'rounded-xl border border-destructive/30 bg-destructive/5 px-3 py-2 text-xs text-destructive',
        statusMessage ? 'block' : 'hidden',
      )}
    >
      {statusMessage}
    </div>
  );
}

/**
 * Application-level toolbar shell for Aideon Desktop.
 * Workspace modules provide `center` (search) and `end` (actions) content.
 * @param root0
 * @param root0.title
 * @param root0.subtitle
 * @param root0.modeLabel
 * @param root0.start
 * @param root0.center
 * @param root0.workspaceToolbar
 * @param root0.end
 * @param root0.statusMessage
 * @param root0.commands
 * @param root0.onShellCommand
 * @param root0.className
 */
export function AideonToolbar({
  title,
  subtitle,
  modeLabel,
  start,
  center,
  workspaceToolbar,
  end,
  statusMessage,
  commands: workspaceCommands = [],
  onShellCommand,
  className,
  ...properties
}: AideonToolbarProperties) {
  const isMac = useIsMacPlatform();
  const shortcutLabelFor = useCallback(
    (letter: string) => (isMac ? `⌘${letter}` : `Ctrl+${letter}`),
    [isMac],
  );
  const sidebar = useOptionalSidebar();
  const shell = useAideonShellControls();
  const isTauri = isTauriRuntime();
  const isDevelopment = isDevelopmentBuild();
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const theme = useOptionalTheme();

  const openStyleguide = useCallback(() => {
    if (isTauri) {
      import('@tauri-apps/api/core')
        .then(({ invoke }) => invoke('open_styleguide'))
        .catch(() => {
          globalThis.location.assign('/styleguide');
        });
      return;
    }
    globalThis.location.assign('/styleguide');
  }, [isTauri]);

  const commands = useMemo(() => {
    const shellCommands = buildShellCommands({
      sidebar,
      shell,
      theme,
      workspaceCommands,
      shortcutLabelFor,
    });
    return [
      ...shellCommands,
      {
        id: 'help.shortcuts',
        group: 'Help',
        label: 'Keyboard shortcuts…',
        onSelect: () => {
          setShortcutsOpen(true);
        },
      },
      ...(isDevelopment
        ? ([
            {
              id: 'debug.styleguide',
              group: 'Debug',
              label: 'UI Style Guide',
              onSelect: () => {
                openStyleguide();
              },
            },
          ] satisfies AideonCommandItem[])
        : []),
    ] satisfies AideonCommandItem[];
  }, [isDevelopment, openStyleguide, shell, sidebar, theme, workspaceCommands, shortcutLabelFor]);

  useBrowserShortcutHandler({
    isTauri,
    sidebar,
    shell,
    openCommandPalette: () => {
      setCommandPaletteOpen(true);
    },
  });

  useTauriShellCommandListener({
    isTauri,
    sidebar,
    shell,
    onShellCommand,
    openCommandPalette: () => {
      setCommandPaletteOpen(true);
    },
  });

  useEffect(() => {
    if (typeof globalThis === 'undefined') {
      return;
    }
    const handleCommandPalette = () => {
      setCommandPaletteOpen(true);
    };
    globalThis.addEventListener('aideon.workspace.open-command-palette', handleCommandPalette);
    return () => {
      globalThis.removeEventListener('aideon.workspace.open-command-palette', handleCommandPalette);
    };
  }, []);

  return (
    <div
      data-tauri-drag-region="false"
      className={cn('flex flex-col gap-2', className)}
      {...properties}
    >
      <Toolbar className="h-12 w-full rounded-2xl px-3 py-2">
        <ToolbarStartSection
          sidebar={sidebar}
          shell={shell}
          isTauri={isTauri}
          isDevelopment={isDevelopment}
          openStyleguide={openStyleguide}
          shortcutLabelFor={shortcutLabelFor}
          start={start}
          title={title}
          subtitle={subtitle}
          modeLabel={modeLabel}
          onOpenCommandPalette={() => {
            setCommandPaletteOpen(true);
          }}
          onOpenShortcuts={() => {
            setShortcutsOpen(true);
          }}
        />

        <ToolbarCenterSection center={center} />

        <ToolbarEndSection end={end} theme={theme} />
      </Toolbar>

      <ToolbarStatusMessage statusMessage={statusMessage} />

      {workspaceToolbar ? <div className="w-full">{workspaceToolbar}</div> : undefined}

      <AideonCommandPalette
        open={commandPaletteOpen}
        onOpenChange={setCommandPaletteOpen}
        commands={commands}
      />

      <KeyboardShortcutsDialog open={shortcutsOpen} onOpenChange={setShortcutsOpen} />
    </div>
  );
}

/**
 * Placeholder menubar for desktop shell actions.
 * @param root0 - Menu properties.
 * @param root0.onOpenCommandPalette - Opens the command palette.
 * @param root0.onOpenShortcuts - Opens the keyboard shortcuts dialog.
 * @param root0.onOpenStyleguide
 * @param root0.showDebugItems
 * @param root0.shortcutLabelFor
 */
function AppMenu({
  onOpenCommandPalette,
  onOpenShortcuts,
  onOpenStyleguide,
  shortcutLabelFor,
  showDebugItems = false,
}: {
  readonly onOpenCommandPalette: () => void;
  readonly onOpenShortcuts: () => void;
  readonly onOpenStyleguide: () => void;
  readonly shortcutLabelFor: (letter: string) => string;
  readonly showDebugItems?: boolean;
}) {
  const sidebar = useOptionalSidebar();
  const shell = useAideonShellControls();

  return (
    <Menubar className="border-none bg-transparent p-0 shadow-none">
      <MenubarMenu>
        <MenubarTrigger className="px-2 py-1 text-sm font-medium">App</MenubarTrigger>
        <MenubarContent>
          <MenubarItem disabled>Preferences…</MenubarItem>
          <MenubarItem disabled>Check for updates…</MenubarItem>
        </MenubarContent>
      </MenubarMenu>
      <MenubarMenu>
        <MenubarTrigger className="px-2 py-1 text-sm font-medium">View</MenubarTrigger>
        <MenubarContent>
          <MenubarItem
            onSelect={() => {
              onOpenCommandPalette();
            }}
          >
            Command palette{' '}
            <span className="ml-auto text-xs text-muted-foreground">{shortcutLabelFor('K')}</span>
          </MenubarItem>
          <MenubarItem
            disabled={!sidebar}
            onSelect={() => {
              sidebar?.toggleSidebar();
            }}
          >
            Toggle navigation
          </MenubarItem>
          <MenubarItem
            disabled={!shell}
            onSelect={() => {
              shell?.toggleInspector();
            }}
          >
            Toggle inspector
          </MenubarItem>
        </MenubarContent>
      </MenubarMenu>
      <MenubarMenu>
        <MenubarTrigger className="px-2 py-1 text-sm font-medium">Help</MenubarTrigger>
        <MenubarContent>
          <MenubarItem
            onSelect={() => {
              onOpenShortcuts();
            }}
          >
            Keyboard shortcuts…
          </MenubarItem>
          {showDebugItems ? (
            <MenubarItem
              onSelect={() => {
                onOpenStyleguide();
              }}
            >
              UI Style Guide
            </MenubarItem>
          ) : undefined}
          <MenubarItem disabled>About Aideon</MenubarItem>
        </MenubarContent>
      </MenubarMenu>
    </Menubar>
  );
}
