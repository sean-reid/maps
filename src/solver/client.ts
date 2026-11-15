import type { Geometry } from "../geometry.ts";
import type { Reply, Request } from "./worker.ts";
import wasmUrl from "./cartogram.wasm?url";

/** Runs the solver off the main thread. Only the newest request's answer is delivered. */
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
        done?.(reply.id === this.next - 1 ? reply.points : undefined);
      }
    };
    this.ready = fetch(wasmUrl)
      .then((res) => {
        if (!res.ok) throw new Error(`the solver did not load (${res.status})`);
        return res.arrayBuffer();
      })
      .then((wasm) => this.send({ kind: "init", wasm, geo }, [wasm]));
  }

  /** Resolves with the solved points, or undefined when a newer request overtook this one. */
  solve(values: Float64Array, warm: boolean): Promise<Float64Array | undefined> {
    const id = this.next++;
    return new Promise((done) => {
      this.waiting.set(id, done);
      this.ready.then(
        () => this.send({ kind: "solve", id, values, warm }),
        (error: unknown) => {
          this.waiting.delete(id);
          this.failed?.(error instanceof Error ? error.message : String(error));
          done(undefined);
        },
      );
    });
  }

  private send(message: Request, transfer: Transferable[] = []): void {
    this.worker.postMessage(message, transfer);
  }
}
