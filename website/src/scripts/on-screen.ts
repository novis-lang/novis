/**
 * Marks each element with a `data-motion` attribute with `data-on-screen`
 * while any part of it is in the window, or within 64px of it.
 *
 * custom.css pauses every animation inside a `[data-motion]` element that has
 * no `data-on-screen`. So a component that loops an animation adds
 * `data-motion` to its root, and the loop runs only while the reader can see
 * it. A paused animation costs nothing, and it goes on from the same frame
 * when it comes back into view. Without this script nothing is marked, and
 * the animations stay still on their first frame.
 *
 * Elements added to the page later are not observed; every animated region
 * on this site is in the page's HTML.
 */
const observer = new IntersectionObserver(
  (entries) => {
    for (const entry of entries) entry.target.toggleAttribute('data-on-screen', entry.isIntersecting)
  },
  { rootMargin: '64px' },
)
for (const el of document.querySelectorAll('[data-motion]')) observer.observe(el)
