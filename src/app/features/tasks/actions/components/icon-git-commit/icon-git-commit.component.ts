import { Component } from "@angular/core";

@Component({
    selector: "app-icon-git-commit",
    standalone: true,
    styles: `
        :host {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            line-height: 0;
            font-size: 16px;
        }
        svg {
            width: 1em;
            height: 1em;
            display: block;
        }
    `,
    template: `
        <svg
            class="action-icon"
            aria-hidden="true"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.6"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <circle cx="12" cy="12" r="3"></circle>
            <path d="M3 12h6"></path>
            <path d="M15 12h6"></path>
            <path d="M7 7l-4 4 4 4"></path>
        </svg>
    `,
})
export class IconGitCommitComponent {}
