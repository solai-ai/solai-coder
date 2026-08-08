import { SolaiAgentOptions } from "./codexOptions";
import { SolaiAgentExec } from "./exec";
import { Thread } from "./thread";
import { ThreadOptions } from "./threadOptions";

/**
 * SolaiAgent is the main class for interacting with the SolaiAgent agent.
 *
 * Use the `startThread()` method to start a new thread or `resumeThread()` to resume a previously started thread.
 */
export class SolaiAgent {
  private exec: SolaiAgentExec;
  private options: SolaiAgentOptions;

  constructor(options: SolaiAgentOptions = {}) {
    const { codexPathOverride, env, config } = options;
    this.exec = new SolaiAgentExec(codexPathOverride, env, config);
    this.options = options;
  }

  /**
   * Starts a new conversation with an agent.
   * @returns A new thread instance.
   */
  startThread(options: ThreadOptions = {}): Thread {
    return new Thread(this.exec, this.options, options);
  }

  /**
   * Resumes a conversation with an agent based on the thread id.
   * Threads are persisted in ~/.codex/sessions.
   *
   * @param id The id of the thread to resume.
   * @returns A new thread instance.
   */
  resumeThread(id: string, options: ThreadOptions = {}): Thread {
    return new Thread(this.exec, this.options, options, id);
  }
}
