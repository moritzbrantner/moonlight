export type Page = "overview" | "dashboard";

export function currentPage(): Page {
  return new URLSearchParams(window.location.search).get("page") === "overview" ? "overview" : "dashboard";
}

export function navigate(page: Page, setPage: (page: Page) => void) {
  const url = new URL(window.location.href);
  if (page === "overview") {
    url.searchParams.set("page", "overview");
  } else {
    url.searchParams.delete("page");
  }
  window.history.pushState({}, "", url);
  setPage(page);
}
