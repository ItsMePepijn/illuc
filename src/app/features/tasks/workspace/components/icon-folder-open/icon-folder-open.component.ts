import { Component } from "@angular/core";

@Component({
    selector: "app-icon-folder-open",
    standalone: true,
    styles: `
        :host {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            line-height: 0;
        }
        svg {
            width: 18px;
            height: 18px;
            display: block;
        }
    `,
    template: `
        <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <path d="M3 8.5A2.5 2.5 0 0 1 5.5 6H10l2 2h6.5A2.5 2.5 0 0 1 21 10.5v6A2.5 2.5 0 0 1 18.5 19h-13A2.5 2.5 0 0 1 3 16.5z"></path>
            <path d="M3 10h18"></path>
        </svg>
    `,
})
export class IconFolderOpenComponent {}
