import { execSync } from "node:child_process";

interface ToolResultContentItem {
  type: string;
  text?: string;
  [key: string]: unknown;
}

interface ToolResultEvent {
  toolName: string;
  toolCallId?: string;
  input?: Record<string, unknown>;
  content?: ToolResultContentItem[];
  details?: unknown;
  isError?: boolean;
}

interface HookUIContext {
  notify?(message: string, level?: "info" | "warning" | "error"): void;
  [key: string]: unknown;
}

interface HookContext {
  cwd: string;
  hasUI?: boolean;
  ui?: HookUIContext;
  [key: string]: unknown;
}

interface HookAPI {
  on(
    event: "tool_result",
    handler: (
      event: ToolResultEvent,
      ctx: HookContext,
    ) =>
      | Promise<{ content?: ToolResultContentItem[]; details?: unknown } | void>
      | { content?: ToolResultContentItem[]; details?: unknown }
      | void,
  ): void;
  [key: string]: unknown;
}

interface ExecError {
  status?: number;
  stdout?: Buffer | string;
  stderr?: Buffer | string;
  message?: string;
}

export default function (pi: HookAPI): void {
  pi.on("tool_result", async (event: ToolResultEvent, ctx: HookContext) => {
    const toolName = event.toolName;
    if (!["edit", "write", "ast_edit"].includes(toolName)) {
      return;
    }

    if (event.isError) {
      return;
    }

    let touchesRust = false;

    if (toolName === "write") {
      const path = event.input?.path;
      if (typeof path === "string" && path.endsWith(".rs")) {
        touchesRust = true;
      }
    } else if (toolName === "edit") {
      const inputStr = event.input?.input;
      if (typeof inputStr === "string") {
        if (
          /\[[^\]\r\n]+\.rs#[A-F0-9]{4}\]/.test(inputStr) ||
          inputStr.includes(".rs")
        ) {
          touchesRust = true;
        }
      }
    } else if (toolName === "ast_edit") {
      const paths = event.input?.paths;
      if (
        Array.isArray(paths) &&
        paths.some((p: unknown) => typeof p === "string" && p.endsWith(".rs"))
      ) {
        touchesRust = true;
      }
    }

    if (!touchesRust) {
      return;
    }

    let output = "";
    try {
      output = execSync(
        "cargo check --all-targets --all-features 2>&1 | grep -E '^error' | head -5",
        {
          cwd: ctx.cwd,
          encoding: "utf8",
          shell: "/bin/sh",
        },
      ).trim();
    } catch (err: unknown) {
      const execErr = err as ExecError;
      // grep exits with 1 when no matches are found; that is a clean check
      if (execErr.status === 1) {
        output = (execErr.stdout ?? "").toString().trim();
      } else {
        // If cargo itself fails or another error occurs, capture stderr/stdout if any
        const stdout = (execErr.stdout ?? "").toString().trim();
        const stderr = (execErr.stderr ?? "").toString().trim();
        output = stdout || stderr;
      }
    }

    if (output.length > 0) {
      if (ctx.hasUI && ctx.ui?.notify) {
        ctx.ui.notify("cargo check reported errors", "error");
      }

      const existingContent = Array.isArray(event.content) ? event.content : [];
      return {
        content: [
          ...existingContent,
          {
            type: "text",
            text: `\n\n[cargo check errors]:\n${output}`,
          },
        ],
      };
    }
  });
}
