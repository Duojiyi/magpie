import React, { useRef, useImperativeHandle, useCallback, useMemo } from 'react';
import { Virtuoso, VirtuosoHandle } from 'react-virtuoso';
import type { ListRange } from 'react-virtuoso';
import type { ClipboardEntry } from "../../../shared/types";
import type { VirtualClipboardListHandle, VirtualClipboardListProps } from "../types";
import { densityListKey } from "../lib/cardDensity";

type VirtuosoListContext = {
    header?: React.ReactNode;
    footer?: React.ReactNode;
    hasMore: boolean;
    isLoading: boolean;
};

const ListHeader = ({ context }: { context?: VirtuosoListContext }) => {
    const header = context?.header;
    return header ? <div className="list-header">{header}</div> : null;
};

const ListFooter = ({ context }: { context?: VirtuosoListContext }) => {
    if (!context) return null;
    const { footer, isLoading, hasMore } = context;
    if (!isLoading && !hasMore) return footer ? <>{footer}</> : null;

    return (
        <>
            <div style={{
                padding: '20px',
                textAlign: 'center',
                opacity: 0.6,
                fontSize: '12px',
                color: 'var(--text-secondary)'
            }}>
                {isLoading ? '加载中...' : '加载更多...'}
            </div>
            {footer}
        </>
    );
};

const VirtualClipboardList = React.forwardRef<VirtualClipboardListHandle, VirtualClipboardListProps>(
    (props, ref) => {
        const {
            items,
            renderItem,
            onLoadMore,
            hasMore,
            isLoading,
            selectedIndex,
            isKeyboardMode,
            onScroll,
            compactMode,
            cardDensity,
            header,
            footer
        } = props;

        const virtuosoRef = useRef<VirtuosoHandle>(null);
        const visibleRangeRef = useRef<ListRange | null>(null);
        // 记录当前滚动位置：用于判断 wheel 事件是否应停止冒泡（见下方 handleWheel）
        const scrollTopRef = useRef(0);
        useImperativeHandle(ref, () => ({
            scrollToItem: (index: number) => {
                virtuosoRef.current?.scrollIntoView({
                    index,
                    behavior: 'smooth',
                    align: 'center',
                });
            },
            scrollToTop: () => {
                virtuosoRef.current?.scrollTo({
                    top: 0,
                    behavior: 'auto'
                });
            },
            resetAfterIndex: (_index: number) => {
                // Not needed with Virtuoso as it handles dynamic heights automatically
            }
        }));

        // 测试钩子：仅当注入测试标志 window.__MAGPIE_TEST__ 时，向全局暴露虚拟列表句柄，
        // 供大列表性能脚手架（C2 / 需求 21）做确定性的滚动定位与条目计数。
        // 生产环境从不设置该标志，effect 直接 return，对生产行为零影响。
        React.useEffect(() => {
            if (typeof window === 'undefined') return;
            const w = window as unknown as {
                __MAGPIE_TEST__?: boolean;
                __magpieVirtualList?: {
                    scrollToIndex: (index: number, align?: 'start' | 'center' | 'end') => void;
                    scrollTo: (top: number) => void;
                    getItemCount: () => number;
                };
            };
            if (!w.__MAGPIE_TEST__) return;
            w.__magpieVirtualList = {
                scrollToIndex: (index, align = 'center') =>
                    virtuosoRef.current?.scrollToIndex({ index, align, behavior: 'auto' }),
                scrollTo: (top) => virtuosoRef.current?.scrollTo({ top, behavior: 'auto' }),
                getItemCount: () => items.length,
            };
            return () => {
                delete w.__magpieVirtualList;
            };
        }, [items.length]);

        // Keep keyboard selection visible even when the item is only in overscan
        React.useEffect(() => {
            if (!isKeyboardMode || selectedIndex < 0) return;

            const range = visibleRangeRef.current;
            const edgeBuffer = 1;

            if (!range) {
                virtuosoRef.current?.scrollToIndex({
                    index: selectedIndex,
                    behavior: 'auto',
                    align: 'center',
                });
                return;
            }

            if (selectedIndex < range.startIndex + edgeBuffer) {
                virtuosoRef.current?.scrollToIndex({
                    index: selectedIndex,
                    behavior: 'auto',
                    align: 'start',
                });
                return;
            }

            if (selectedIndex > range.endIndex - edgeBuffer) {
                virtuosoRef.current?.scrollToIndex({
                    index: selectedIndex,
                    behavior: 'auto',
                    align: 'end',
                });
            }
        }, [selectedIndex, isKeyboardMode]);


        // Handle scroll events
        const handleScroll = useCallback((scrollTop: number) => {
            scrollTopRef.current = scrollTop;
            onScroll?.(scrollTop);
        }, [onScroll]);

        // 滚轮事件处理（U3 / 需求 12.1、12.2）
        // 固定窗口模式下后端钩子已把滚轮转发给本 webview，事件会落到可滚动的 Virtuoso scroller 上。
        // 这里在列表“已滚离顶部”时阻止 wheel 冒泡，避免被上层 <main> 的 handleMainWheel
        // 误处理（其仅在列表顶部用于唤出/收起搜索栏）；列表处于顶部时仍放行冒泡以保留搜索栏显隐手势。
        const handleWheel = useCallback((e: React.WheelEvent<HTMLDivElement>) => {
            if (scrollTopRef.current > 0) {
                e.stopPropagation();
            }
        }, []);

        // Handle end reached for infinite loading
        const handleEndReached = useCallback(() => {
            if (hasMore && !isLoading && onLoadMore) {
                onLoadMore();
            }
        }, [hasMore, isLoading, onLoadMore]);

        const handleRangeChanged = useCallback((range: ListRange) => {
            visibleRangeRef.current = range;
        }, []);

        // Memoized item renderer for Virtuoso
        const itemContent = useCallback((index: number, item: ClipboardEntry) => {
            return (
                <div style={{ paddingBottom: compactMode ? 2 : 4 }}>
                    {renderItem(item, index, index === 0)}
                </div>
            );
        }, [renderItem, compactMode]);

        const components = useMemo(() => ({
            Header: ListHeader,
            Footer: ListFooter
        }), []);

        const context = useMemo(() => ({
            header,
            footer,
            hasMore,
            isLoading
        }), [header, footer, hasMore, isLoading]);

        return (
            <div className="virtual-list-wrapper" style={{ height: '100%', width: '100%' }} onWheel={handleWheel}>
                <Virtuoso
                    // 密度切换时通过 key 变化强制重挂载 Virtuoso，触发行高全量重算（V5 / 需求 32.3）。
                    // Virtuoso 会缓存已测量的行高，仅靠 CSS 改变高度不会刷新缓存的偏移；重挂载可保证渲染正确。
                    key={densityListKey(cardDensity, !!compactMode)}
                    ref={virtuosoRef}
                    data={items}
                    itemContent={itemContent}
                    components={components}
                    context={context}
                    style={{ height: '100%' }}
                    onScroll={(e) => handleScroll((e.currentTarget as HTMLElement).scrollTop)}
                    endReached={handleEndReached}
                    rangeChanged={handleRangeChanged}
                    overscan={200} // Pre-render 200px of content for smoother scrolling
                />
            </div>
        );
    }
);

VirtualClipboardList.displayName = 'VirtualClipboardList';

export { VirtualClipboardList };
export default VirtualClipboardList;
