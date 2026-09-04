// Diff text for the result screen. Written here so the screen file stays about layout.
export const hunks: Record<string, string> = {
  "backend/staff-react/src/media/UploadTile.tsx": `@@ -18,14 +18,24 @@ export function UploadTile({ item }: { item: QueueItem }) {
-  const done = item.progress === 100
+  // The upload finishing is not the same as the row having a committed snapshot.
+  const done = item.committedAt !== null

   return (
     <div
       className="tile"
-      style={{ order: done ? 0 : 1 }}
+      data-committed={done}
       onClick={() => open(item.id)}
     >
-      {done ? <img src={item.url} /> : <Spinner />}
+      {item.url ? (
+        <img src={item.url} alt="" className="tile-photo tile-photo-in" />
+      ) : (
+        <Spinner label="Uploading" />
+      )}
     </div>
   )
 }`,
  "backend/staff-react/src/media/useUploadQueue.ts": `@@ -41,9 +41,20 @@ export function useUploadQueue(vehicleId: string) {
-      setItems((prev) => sortByProgress(prev))
+      // Keep the order the grid was rendered with until the server confirms.
+      setItems((prev) => prev.map((i) => (i.id === id ? { ...i, url } : i)))
     })

-  return { items, add }
+  const commit = (id: string, committedAt: string) =>
+    setItems((prev) => prev.map((i) => (i.id === id ? { ...i, committedAt } : i)))
+
+  return { items, add, commit }`,
  "backend/staff-react/src/media/UploadTile.test.tsx": `@@ -30,3 +30,8 @@ describe("UploadTile", () => {
+  it("keeps its position until the snapshot commits", () => {
+    const { rerender } = render(<UploadTile item={uploading} />)
+    const before = screen.getByRole("img").closest(".tile")
+    rerender(<UploadTile item={{ ...uploading, url: "/p.jpg" }} />)
+    expect(before).toBe(screen.getByRole("img").closest(".tile"))
+  })`,
}
