import { Component } from "@angular/core";

@Component({
    selector: "app-icon-git-merge",
    standalone: true,
    styles: `
        :host {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            line-height: 0;
            font-size: var(--font-size-md);
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
            <circle cx="7" cy="6" r="2.25"></circle>
            <circle cx="17" cy="18" r="2.25"></circle>
            <path d="M9.25 6h2.25a4.5 4.5 0 0 1 4.5 4.5V15"></path>
            <path d="M13 12l3 3 3-3"></path>
        </svg>
    `,
})
export class IconGitMergeComponent {}
