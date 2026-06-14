import { RefreshCw } from "lucide-react";
import { navigate, type Page } from "../navigation";

type TopbarProps = {
  page: Page;
  onNavigate: (page: Page) => void;
  onRefresh: () => Promise<void>;
};

export function Topbar({ page, onNavigate, onRefresh }: TopbarProps) {
  return (
    <header className="topbar">
      <button className="brand" onClick={() => navigate("overview", onNavigate)} aria-label="Moonlight overview">
        <span className="brand__mark" aria-hidden="true">ML</span>
        <span>Moonlight</span>
      </button>
      <nav className="top-actions" aria-label="Pages">
        <button className={`nav-button ${page === "overview" ? "active" : ""}`} onClick={() => navigate("overview", onNavigate)} aria-current={page === "overview" ? "page" : undefined}>
          Overview
        </button>
        <button className={`nav-button ${page === "dashboard" ? "active" : ""}`} onClick={() => navigate("dashboard", onNavigate)} aria-current={page === "dashboard" ? "page" : undefined}>
          Dashboard
        </button>
        {page === "dashboard" && (
          <button className="icon-button" onClick={() => void onRefresh()} title="Refresh data" aria-label="Refresh data">
            <RefreshCw size={18} />
          </button>
        )}
      </nav>
    </header>
  );
}
