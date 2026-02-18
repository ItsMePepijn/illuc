import { Component } from "@angular/core";

@Component({
    selector: "app-icon-git-push",
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
            <path d="M8 19v-4h8v4"></path>
            <path d="M12 13V4"></path>
            <path d="M8.5 7.5l3.5-3.5 3.5 3.5"></path>
        </svg>
    `,
})
export class IconGitPushComponent {}
