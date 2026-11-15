import type { Geometry } from "../geometry.ts";
import { createSolver, type Solver } from "./glue.ts";

export type Request =
  | { kind: "init"; wasm: ArrayBuffer; geo: Geometry }
  | { kind: "solve"; id: number; values: Float64Array; warm: boolean };

export type Reply =
  { id: number; points: Float64Array; meanError: number } | { id: number; error: string };

let solver: Promise<Solver> | undefined;
let last: Float64Array | undefined;

self.onmessage = async (event: MessageEvent<Request>) => {
  const message = event.data;
  if (message.kind === "init") {
    solver = createSolver(message.wasm, message.geo);
    return;
  }
  const post = (reply: Reply, transfer: Transferable[] = []) =>
    (self as unknown as Worker).postMessage(reply, transfer);
  try {
    if (!solver) throw new Error("solver was not started");
    const { points, meanError } = (await solver).solve(
      message.values,
      message.warm ? last : undefined,
    );
    last = points.slice();
    post({ id: message.id, points, meanError }, [points.buffer]);
  } catch (error) {
    post({ id: message.id, error: error instanceof Error ? error.message : String(error) });
  }
};
