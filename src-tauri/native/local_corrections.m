#import <AppKit/AppKit.h>
#import <ApplicationServices/ApplicationServices.h>

// This bridge holds only the field in which Handy just dictated. No global
// keyboard hooks, screenshots, clipboard polling, or persistence live here.
@interface LCField : NSObject
@property AXUIElementRef element;
@property pid_t pid;
@end
@implementation LCField
- (void)dealloc { if (_element) CFRelease(_element); }
@end

static id attribute(AXUIElementRef element, CFStringRef name) {
    CFTypeRef value = NULL;
    if (AXUIElementCopyAttributeValue(element, name, &value) != kAXErrorSuccess) return nil;
    return CFBridgingRelease(value);
}
static NSString *fieldText(AXUIElementRef element) {
    id role = attribute(element, kAXSubroleAttribute);
    if ([role isEqual:@"AXSecureTextField"]) return nil;
    NSNumber *count = attribute(element, kAXNumberOfCharactersAttribute);
    if (count && count.integerValue > 12000) return nil;
    id value = attribute(element, kAXValueAttribute);
    if (![value isKindOfClass:NSString.class] || [value length] > 12000) return nil;
    return value;
}
static char *json(NSDictionary *value) {
    NSData *data = [NSJSONSerialization dataWithJSONObject:value options:0 error:nil];
    if (!data) return NULL;
    return strndup(data.bytes, data.length);
}
// Status codes expose only failure categories, never field contents or app names.
// 1: focus changed; 2: focus unavailable; 3: text unavailable;
// 4: selection unavailable; 5: invalid selection; 6: serialization;
// 7: own app or invalid owner.
void *lc_capture(int *status) {
    *status = 2;
    @autoreleasepool {
        AXUIElementRef system = AXUIElementCreateSystemWide();
        AXUIElementSetMessagingTimeout(system, 0.2);
        id focused = attribute(system, kAXFocusedUIElementAttribute);
        CFRelease(system);
        if (!focused || CFGetTypeID((__bridge CFTypeRef)focused) != AXUIElementGetTypeID()) return NULL;
        AXUIElementRef element = (__bridge AXUIElementRef)focused;
        AXUIElementSetMessagingTimeout(element, 0.2);
        pid_t pid = 0;
        if (AXUIElementGetPid(element, &pid) != kAXErrorSuccess || pid == getpid()) { *status = 7; return NULL; }
        if (!fieldText(element)) { *status = 3; return NULL; }
        LCField *field = [LCField new];
        field.element = (AXUIElementRef)CFRetain(element);
        field.pid = pid;
        *status = 0;
        return (__bridge_retained void *)field;
    }
}
char *lc_snapshot(void *handle, bool requireFocus, int *status) {
    *status = 2;
    @autoreleasepool {
        LCField *field = (__bridge LCField *)handle;
        if (!field) return NULL;
        if (requireFocus) {
            AXUIElementRef system = AXUIElementCreateSystemWide();
            AXUIElementSetMessagingTimeout(system, 0.2);
            id focused = attribute(system, kAXFocusedUIElementAttribute);
            CFRelease(system);
            if (!focused || CFGetTypeID((__bridge CFTypeRef)focused) != AXUIElementGetTypeID()) return NULL;
            if (!CFEqual((__bridge CFTypeRef)focused, field.element)) { *status = 1; return NULL; }
        }
        NSString *text = fieldText(field.element);
        if (!text) { *status = 3; return NULL; }
        id selected = attribute(field.element, kAXSelectedTextRangeAttribute);
        CFRange range;
        if (!selected || CFGetTypeID((__bridge CFTypeRef)selected) != AXValueGetTypeID() ||
            !AXValueGetValue((__bridge AXValueRef)selected, kAXValueCFRangeType, &range)) { *status = 4; return NULL; }
        if (range.location < 0 || range.length < 0 || range.location + range.length > text.length) { *status = 5; return NULL; }
        char *result = json(@{@"text":text, @"start":@(range.location), @"length":@(range.length)});
        *status = result ? 0 : 6;
        return result;
    }
}
bool lc_replace(void *handle, const char *expectedUTF8, long start, long length, const char *replacementUTF8) {
    @autoreleasepool {
        LCField *field = (__bridge LCField *)handle;
        NSString *expected = [NSString stringWithUTF8String:expectedUTF8];
        NSString *replacement = [NSString stringWithUTF8String:replacementUTF8];
        if (!field || !expected || !replacement || ![fieldText(field.element) isEqual:expected] || start < 0 || length < 0 || start + length > expected.length) return false;
        Boolean canSet = false;
        if (AXUIElementIsAttributeSettable(field.element, kAXSelectedTextAttribute, &canSet) != kAXErrorSuccess || !canSet) return false;
        CFRange range = CFRangeMake(start, length);
        AXValueRef selected = AXValueCreate(kAXValueCFRangeType, &range);
        AXError error = AXUIElementSetAttributeValue(field.element, kAXSelectedTextRangeAttribute, selected);
        CFRelease(selected);
        if (error != kAXErrorSuccess) return false;
        error = AXUIElementSetAttributeValue(field.element, kAXSelectedTextAttribute, (__bridge CFStringRef)replacement);
        if (error != kAXErrorSuccess) return false;
        NSString *updated = [expected stringByReplacingCharactersInRange:NSMakeRange(start, length) withString:replacement];
        return [fieldText(field.element) isEqual:updated];
    }
}
void lc_release(void *handle) { if (handle) CFRelease(handle); }
void lc_free(char *string) { free(string); }
