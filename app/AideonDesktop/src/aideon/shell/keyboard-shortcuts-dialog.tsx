import type { ReactElement } from 'react';

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from 'design-system/components/ui/dialog';
import { Kbd } from 'design-system/components/ui/kbd';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from 'design-system/components/ui/table';

export interface ShortcutRow {
  readonly category: string;
  readonly name: string;
  readonly keys: readonly string[];
}

export interface KeyboardShortcutsDialogProperties {
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}

/**
 * Default shortcut set for the shell. Keep in sync with Tauri menu accelerators.
 * @returns Shortcut rows.
 */
function defaultShortcuts(): ShortcutRow[] {
  return [
    { category: 'File', name: 'Open…', keys: ['CmdOrCtrl', 'O'] },
    { category: 'File', name: 'Save As…', keys: ['CmdOrCtrl', 'Shift', 'S'] },
    { category: 'File', name: 'Print…', keys: ['CmdOrCtrl', 'P'] },
    { category: 'Shell', name: 'Command palette', keys: ['CmdOrCtrl', 'K'] },
    { category: 'Shell', name: 'Toggle navigation', keys: ['CmdOrCtrl', 'B'] },
    { category: 'Shell', name: 'Toggle inspector', keys: ['CmdOrCtrl', 'I'] },
  ];
}

/**
 * Lightweight keyboard shortcuts reference for the desktop shell.
 * @param root0 - Component props.
 * @param root0.open - Whether the dialog is open.
 * @param root0.onOpenChange - Open state setter.
 * @returns Dialog UI.
 */
export function KeyboardShortcutsDialog({
  open,
  onOpenChange,
}: KeyboardShortcutsDialogProperties): ReactElement {
  const shortcuts = defaultShortcuts();
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-[680px]">
        <DialogHeader>
          <DialogTitle>Keyboard shortcuts</DialogTitle>
          <DialogDescription>Common shortcuts in Aideon Desktop.</DialogDescription>
        </DialogHeader>
        <div className="rounded-lg border border-border/70">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-[140px]">Category</TableHead>
                <TableHead>Action</TableHead>
                <TableHead className="w-[220px] text-right">Shortcut</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {shortcuts.map((row) => (
                <TableRow key={`${row.category}:${row.name}`}>
                  <TableCell className="text-xs text-muted-foreground">{row.category}</TableCell>
                  <TableCell className="font-medium">{row.name}</TableCell>
                  <TableCell className="text-right">
                    <span className="inline-flex items-center gap-1">
                      {row.keys.map((key) => (
                        <Kbd key={key}>{key}</Kbd>
                      ))}
                    </span>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      </DialogContent>
    </Dialog>
  );
}
