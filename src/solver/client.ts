import type { Geometry } from "../geometry.ts";
import type { Reply, Request } from "./worker.ts";
import wasmUrl from "./cartogram.wasm?url";

/** Runs the solver off the main thread. */
export class SolverClient {
  private worker: Worker;
  private next = 1;
  private waiting = new Map<number, (points: Float64Array | undefined) => void>();
  private failed: ((message: string) => void) | undefined;
  /** Settles once the worker has been handed the WASM; solves wait for it. */
  private ready: Promise<void>;

  constructor(geo: Geometry, onError?: (message: string) => void) {
    this.failed = onError;
    this.worker = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
    this.worker.onmessage = (event: MessageEvent<Reply>) => {
      const reply = event.data;
      const done = this.waiting.get(reply.id);
      this.waiting.delete(reply.id);
      if ("error" in reply) {
        this.failed?.(reply.error);
        done?.(undefined);
      } else {
        done?.(reply.points);
      }
    };
    this.ready = fetch(wasmUrl)
      .then((res) => {
        if (!res.ok) throw new Error(`the solver did not load (${res.status})`);
        return res.arrayBuffer();
      })
      .then((wasm) => this.send({ kind: "init", wasm, geo }, [wasm]));
    // A failed load is reported by the first solve; this keeps it from surfacing unhandled before then.
    this.ready.catch(() => undefined);
  }

  /**
   * Resolves with the solved points, or undefined when a newer request overtook this one.
   * One solve runs at a time; of those asked for meanwhile, only the newest is sent.
   */
  solve(values: Float64Array, warm: boolean): Promise<Float64Array | undefined> {
    return new Promise((done) => {
      this.queued?.done(undefined);
      this.queued = { values, warm, done };
      this.ready.then(
        () => this.pump(),
        (error: unknown) => {
          this.queued = undefined;
          this.failed?.(error instanceof Error ? error.message : String(error));
          done(undefined);
        },
      );
    });
  }

  private queued:
    | { values: Float64Array; warm: boolean; done: (points: Float64Array | undefined) => void }
    | undefined;

  private pump(): void {
    if (this.waiting.size > 0 || !this.queued) return;
    const { values, warm, done } = this.queued;
    this.queued = undefined;
    const id = this.next++;
    this.waiting.set(id, (points) => {
      done(points);
      this.pump();
    });
    this.send({ kind: "solve", id, values, warm });
  }

  private send(message: Request, transfer: Transferable[] = []): void {
    this.worker.postMessage(message, transfer);
  }
}
