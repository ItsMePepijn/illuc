import { Component } from "@angular/core";

@Component({
    selector: "app-icon-stop-square",
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
            fill="currentColor"
        >
            <rect x="7" y="7" width="10" height="10" rx="1"></rect>
        </svg>
    `,
})
export class IconStopSquareComponent {}
