import { components } from '@/api/schema.gen';

// Types for grouped streaming content
export interface ContentGroup {
    type: 'content';
    content: string;
    index: number;
}

export interface ToolGroup {
    type: 'tool';
    toolCall: components['schemas']['ToolCallEntry'];
    index: number;
}

export type StreamingGroup = ContentGroup | ToolGroup;

/**
 * Process streaming data to create properly ordered and grouped content.
 * Maintains the original order while grouping adjacent items of the same type.
 *
 * Example:
 * Input: [toolstart, toolresult, msgdelta, msgdelta, msgdelta, toolstart, toolresult, msgdelta]
 * Output: [tool1, content1, tool2, content2]
 */
export function processStreamingData(
    data: components['schemas']['StreamingResponse'][]
): StreamingGroup[] {
    const groups: StreamingGroup[] = [];
    const toolCallsMap = new Map<string, components['schemas']['ToolCallEntry']>();
    const toolCallPositions = new Map<string, number>(); // Track where each tool call should be positioned
    let currentContentChunk = '';

    for (let i = 0; i < data.length; i++) {
        const response = data[i];

        if (response.entry_type === 'Content' && response.content) {
            // Accumulate content
            currentContentChunk += response.content;

            // Check if the next entry is also content - if not, we need to finalize this content group
            const nextEntry = data[i + 1];
            const isLastEntry = i === data.length - 1;
            const nextIsNotContent = nextEntry && nextEntry.entry_type !== 'Content';

            if (isLastEntry || nextIsNotContent) {
                // Finalize the current content chunk
                if (currentContentChunk.trim()) {
                    groups.push({
                        type: 'content',
                        content: currentContentChunk,
                        index: groups.length,
                    });
                    currentContentChunk = '';
                }
            }
        } else if (response.tool_call && response.entry_type !== 'Content') {
            // Handle tool calls
            const toolId = response.tool_call.tool_id;
            const existing = toolCallsMap.get(toolId);

            // Update with the latest information, preserving important fields
            const updatedToolCall: components['schemas']['ToolCallEntry'] = {
                ...response.tool_call,
                // Keep arguments from the first entry if current one doesn't have them
                arguments: response.tool_call.arguments || existing?.arguments,
                // Always update result if present
                result: response.tool_call.result || existing?.result,
                // Use the most advanced status (Success > Executing > Starting)
                status: getHighestPriorityStatus(response.tool_call.status, existing?.status),
            };

            toolCallsMap.set(toolId, updatedToolCall);

            // If this is the first time we see this tool call, add it to the groups
            if (!existing) {
                const position = groups.length;

                toolCallPositions.set(toolId, position);
                groups.push({
                    type: 'tool',
                    toolCall: updatedToolCall,
                    index: position,
                });
            } else {
                // Update the existing tool call in the groups array
                const position = toolCallPositions.get(toolId);

                if (position !== undefined && position < groups.length) {
                    const group = groups[position];

                    if (group.type === 'tool') {
                        group.toolCall = updatedToolCall;
                    }
                }
            }
        }
    }

    return groups;
}

// Helper function to determine the highest priority status
function getHighestPriorityStatus(
    newStatus?: components['schemas']['ToolCallEntry']['status'],
    existingStatus?: components['schemas']['ToolCallEntry']['status']
): components['schemas']['ToolCallEntry']['status'] {
    const priorities = {
        Starting: 1,
        Executing: 2,
        Success: 3,
        Error: 3, // Error and success have equal priority (both are final states)
    };

    if (!newStatus) return existingStatus || 'Starting';

    if (!existingStatus) return newStatus;

    const newPriority = priorities[newStatus as keyof typeof priorities] || 1;
    const existingPriority = priorities[existingStatus as keyof typeof priorities] || 1;

    return newPriority >= existingPriority ? newStatus : existingStatus;
}
