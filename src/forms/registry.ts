export interface FormInfo {
  id: string;
  title: string;
  text: string;
  group: "Always on screen" | "Attached to an edge" | "Playful" | "On demand";
}

/** Every on-screen form, in the order the gallery shows them. `tray` and `confetti` are not windows. */
export const FORM_INFO: FormInfo[] = [
  { id: "island", title: "Island", text: "A small pill at the top of the screen. Hover it to see every number.", group: "Always on screen" },
  { id: "bar", title: "Floating bar", text: "One slim bar with all your numbers side by side.", group: "Always on screen" },
  { id: "pills", title: "Taskbar pills", text: "Separate little pills lined up along the bottom.", group: "Always on screen" },
  { id: "orb", title: "Corner orb", text: "A round bubble in a corner. Hover it to open the list.", group: "Always on screen" },
  { id: "ticker", title: "Ticker", text: "A news-style band that scrolls through your numbers.", group: "Always on screen" },
  { id: "trend", title: "Trend bar", text: "Numbers with a small chart of the last day.", group: "Always on screen" },
  { id: "chips", title: "Sticky chips", text: "Colourful sticky notes, one per number.", group: "Always on screen" },
  { id: "versus", title: "Versus", text: "Two numbers face off on a scoreboard.", group: "Always on screen" },
  { id: "stream", title: "Stream overlay", text: "Big, bold numbers made for OBS and live streams.", group: "Always on screen" },
  { id: "edge", title: "Edge tab", text: "A slim tab on the side of the screen. Hover it to slide out the details.", group: "Attached to an edge" },
  { id: "drawer", title: "Pull-out drawer", text: "A handle on the screen edge that pulls out a drawer.", group: "Attached to an edge" },
  { id: "notch", title: "Welded notch", text: "Grows out of the top edge like a camera notch.", group: "Attached to an edge" },
  { id: "wings", title: "Notch wings", text: "Two wings on either side of the top centre.", group: "Attached to an edge" },
  { id: "toptabs", title: "Top tabs", text: "Small tabs hanging from the top edge.", group: "Attached to an edge" },
  { id: "folders", title: "Folder tabs", text: "Folder-style tabs along the top of the screen.", group: "Attached to an edge" },
  { id: "deck", title: "Taskbar deck", text: "A strip that sits right on top of the taskbar.", group: "Attached to an edge" },
  { id: "tag", title: "Hanging tag", text: "A price tag on a string that swings when the number moves.", group: "Playful" },
  { id: "clothesline", title: "Clothesline", text: "Cards pegged to a line across the top.", group: "Playful" },
  { id: "train", title: "Train", text: "A little train, one wagon per number.", group: "Playful" },
  { id: "pet", title: "Desktop pet", text: "A cow that tells you the news in a speech bubble.", group: "Playful" },
  { id: "pager", title: "Pager", text: "A retro pager that beeps through your numbers.", group: "Playful" },
  { id: "pop", title: "Pop-up", text: "Stays hidden until a number changes, then shows it for 4 seconds.", group: "On demand" },
  { id: "feed", title: "Change feed", text: "A running list of the latest changes.", group: "On demand" },
  { id: "peek", title: "Quick peek", text: "Press the quick peek shortcut to see everything for a few seconds.", group: "On demand" },
  { id: "tray", title: "Tray number", text: "One number next to the Cownter icon in the menu bar or system tray.", group: "On demand" },
  { id: "confetti", title: "Goal confetti", text: "Celebrate when a goal is reached or a number sets a new record.", group: "On demand" },
];

export const BOARD_IDS = FORM_INFO.map((f) => f.id).filter((id) => !["island", "edge", "pop", "tray", "confetti"].includes(id));
