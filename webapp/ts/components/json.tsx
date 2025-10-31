import React from "react";
import {Light as SyntaxHighlighter} from 'react-syntax-highlighter';
import json from 'react-syntax-highlighter/dist/esm/languages/hljs/json';
import lightTheme from 'react-syntax-highlighter/dist/esm/styles/hljs/a11y-light';
import darkTheme from 'react-syntax-highlighter/dist/esm/styles/hljs/a11y-dark';

SyntaxHighlighter.registerLanguage('json', json);

export function Json({json, format, theme}: { json: any, format?: boolean, theme?: "light" | "dark" }) {
    format = format === undefined ? true : format;
    theme = theme === undefined ? "light" : theme;
    return <SyntaxHighlighter language={"json"} wrapLongLines={true} style={theme === "light" ? lightTheme : darkTheme}
                              customStyle={{margin: 0, padding: 0}}>
        {JSON.stringify(json, null, format ? 2 : 0)}
    </SyntaxHighlighter>;
}