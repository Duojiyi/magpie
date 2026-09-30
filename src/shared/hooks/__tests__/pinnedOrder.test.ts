import { describe, it, expect } from "vitest";
import { computePinnedOrders } from "../usePinnedSort";
import type { ClipboardEntry } from "../../types";

const pin = (id: number, pinned_order: number, timestamp = id) =>
  ({ id, pinned_order, timestamp, is_pinned: true }) as ClipboardEntry;

describe("computePinnedOrders", () => {
  it("reordering a filtered subset reuses its own values and leaves hidden items alone", () => {
    // Display order A(5) B(4) C(3) D(2); only A and C are visible and get swapped.
    const history = [pin(1, 5), pin(2, 4), pin(3, 3), pin(4, 2)];
    const orders = computePinnedOrders(history, [3, 1]);
    expect([...orders]).toEqual([
      [3, 5],
      [1, 3],
    ]);
  });

  it("renumbers every pinned item when the visible values are not distinct", () => {
    // All legacy 0 orders: display order is by timestamp desc -> 4 3 2 1.
    const history = [pin(1, 0), pin(2, 0), pin(3, 0), pin(4, 0)];
    const orders = computePinnedOrders(history, [1, 3]);
    // Visible slots (3 at #2, 1 at #4) are filled with 1 then 3: 4 1 2 3.
    expect(orders.get(4)).toBe(4);
    expect(orders.get(1)).toBe(3);
    expect(orders.get(2)).toBe(2);
    expect(orders.get(3)).toBe(1);
  });
});
