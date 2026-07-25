import type { ReactNode } from "react";

type Tone = "mauve" | "auburn" | "digital";

// `tone` tints the header-toolbar groups: mauve for the library + nostr
// groups, auburn for the db group, digital (cyan) for the stats group —
// so each cluster reads as distinct at a glance.
const TONE_CLS: Record<Tone, string> = {
  mauve: "bg-mauve/15 text-mauve hover:bg-mauve hover:text-bg",
  auburn: "bg-auburn/15 text-auburn hover:bg-auburn hover:text-bg",
  digital: "bg-digital/15 text-digital hover:bg-digital hover:text-bg",
};

// Pressed (active-toggle) variant — the button stays filled while its
// corresponding view is active. Mirrors the FilterToggle filled-state
// vocabulary so the visual language reads as "currently engaged".
const TONE_PRESSED_CLS: Record<Tone, string> = {
  mauve: "bg-mauve text-bg hover:bg-mauve/80",
  auburn: "bg-auburn text-bg hover:bg-auburn/80",
  digital: "bg-digital text-bg hover:bg-digital/80",
};

interface Props {
  title: string;
  onClick: () => void;
  children: ReactNode;
  tone?: Tone;
  disabled?: boolean;
  pressed?: boolean;
}

// Shared icon-only button for the header toolbar (library / db / nostr / stats).
export function ToolbarIconButton({
  title,
  onClick,
  children,
  tone = "mauve",
  disabled = false,
  pressed = false,
}: Props) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={title}
      aria-label={title}
      aria-pressed={pressed}
      disabled={disabled}
      className={
        "p-2 rounded-md transition-colors disabled:opacity-40 " +
        "disabled:cursor-not-allowed " +
        (pressed ? TONE_PRESSED_CLS[tone] : TONE_CLS[tone])
      }
    >
      {children}
    </button>
  );
}
