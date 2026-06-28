#include <vulkan/vulkan.h>
#include <vulkan/vk_layer.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if defined(_WIN32)
#define VL_EXPORT __declspec(dllexport)
#else
#define VL_EXPORT __attribute__((visibility("default")))
#endif

#define DISP_KEY(obj) (*(void **)(obj))

typedef struct DeviceData {
    void *key;
    VkPhysicalDevice phys;
    PFN_vkGetDeviceProcAddr gdpa;
    PFN_vkCreateSwapchainKHR create_swapchain;
    PFN_vkDestroyDevice destroy_device;
    struct DeviceData *next;
} DeviceData;

static DeviceData *g_devices = NULL;
static PFN_vkGetInstanceProcAddr g_next_instance_gipa = NULL;
static VkInstance g_instance = VK_NULL_HANDLE;
static PFN_vkGetPhysicalDeviceSurfacePresentModesKHR g_get_present_modes = NULL;
static PFN_vkGetPhysicalDeviceSurfaceCapabilitiesKHR g_get_caps = NULL;

static VkPresentModeKHR target_present_mode(void) {
    const char *e = getenv("VORTSTRAP_PRESENT_MODE");
    if (e && *e) {
        int v = atoi(e);
        if (v >= 0 && v <= 3) {
            return (VkPresentModeKHR)v;
        }
    }
    return VK_PRESENT_MODE_IMMEDIATE_KHR;
}

static DeviceData *find_device(void *key) {
    for (DeviceData *d = g_devices; d; d = d->next) {
        if (d->key == key) return d;
    }
    return NULL;
}

static void resolve_wsi(void) {
    if (!g_get_present_modes && g_next_instance_gipa && g_instance) {
        g_get_present_modes = (PFN_vkGetPhysicalDeviceSurfacePresentModesKHR)
            g_next_instance_gipa(g_instance, "vkGetPhysicalDeviceSurfacePresentModesKHR");
        g_get_caps = (PFN_vkGetPhysicalDeviceSurfaceCapabilitiesKHR)
            g_next_instance_gipa(g_instance, "vkGetPhysicalDeviceSurfaceCapabilitiesKHR");
    }
}

static int mode_supported(VkPresentModeKHR m, const VkPresentModeKHR *list, uint32_t n) {
    for (uint32_t i = 0; i < n; i++) {
        if (list[i] == m) return 1;
    }
    return 0;
}

static VkPresentModeKHR choose_present_mode(VkPhysicalDevice phys, VkSurfaceKHR surface) {
    VkPresentModeKHR preferred = target_present_mode();
    resolve_wsi();
    if (!g_get_present_modes) return preferred;
    uint32_t n = 0;
    if (g_get_present_modes(phys, surface, &n, NULL) != VK_SUCCESS || n == 0) return preferred;
    if (n > 32) n = 32;
    VkPresentModeKHR avail[32];
    if (g_get_present_modes(phys, surface, &n, avail) != VK_SUCCESS) return preferred;
    if (mode_supported(preferred, avail, n)) return preferred;
    VkPresentModeKHR fallback[] = {
        VK_PRESENT_MODE_MAILBOX_KHR,
        VK_PRESENT_MODE_IMMEDIATE_KHR,
        VK_PRESENT_MODE_FIFO_RELAXED_KHR,
        VK_PRESENT_MODE_FIFO_KHR,
    };
    for (unsigned i = 0; i < sizeof(fallback) / sizeof(fallback[0]); i++) {
        if (mode_supported(fallback[i], avail, n)) return fallback[i];
    }
    return VK_PRESENT_MODE_FIFO_KHR;
}

static VKAPI_ATTR VkResult VKAPI_CALL CreateSwapchainKHR(
    VkDevice device, const VkSwapchainCreateInfoKHR *pCreateInfo,
    const VkAllocationCallbacks *pAllocator, VkSwapchainKHR *pSwapchain) {
    DeviceData *d = find_device(DISP_KEY(device));
    if (!d || !d->create_swapchain) return VK_ERROR_INITIALIZATION_FAILED;
    VkSwapchainCreateInfoKHR info = *pCreateInfo;
    info.presentMode = choose_present_mode(d->phys, info.surface);
    if (info.presentMode == VK_PRESENT_MODE_MAILBOX_KHR ||
        info.presentMode == VK_PRESENT_MODE_IMMEDIATE_KHR) {
        uint32_t want = 3;
        VkSurfaceCapabilitiesKHR caps;
        if (g_get_caps && g_get_caps(d->phys, info.surface, &caps) == VK_SUCCESS) {
            if (want < caps.minImageCount) want = caps.minImageCount;
            if (caps.maxImageCount != 0 && want > caps.maxImageCount) want = caps.maxImageCount;
        }
        if (info.minImageCount < want) info.minImageCount = want;
    }
    if (getenv("VORTSTRAP_PRESENT_DEBUG")) {
        fprintf(stderr, "[vortstrap] swapchain present mode -> %d, images %u\n",
                (int)info.presentMode, info.minImageCount);
    }
    return d->create_swapchain(device, &info, pAllocator, pSwapchain);
}

static VKAPI_ATTR void VKAPI_CALL DestroyDevice(
    VkDevice device, const VkAllocationCallbacks *pAllocator) {
    void *key = DISP_KEY(device);
    PFN_vkDestroyDevice destroy = NULL;
    DeviceData **pp = &g_devices;
    while (*pp) {
        if ((*pp)->key == key) {
            DeviceData *dead = *pp;
            destroy = dead->destroy_device;
            *pp = dead->next;
            free(dead);
            break;
        }
        pp = &(*pp)->next;
    }
    if (destroy) destroy(device, pAllocator);
}

static VKAPI_ATTR VkResult VKAPI_CALL CreateDevice(
    VkPhysicalDevice physicalDevice, const VkDeviceCreateInfo *pCreateInfo,
    const VkAllocationCallbacks *pAllocator, VkDevice *pDevice) {
    VkLayerDeviceCreateInfo *link = (VkLayerDeviceCreateInfo *)pCreateInfo->pNext;
    while (link &&
           !(link->sType == VK_STRUCTURE_TYPE_LOADER_DEVICE_CREATE_INFO &&
             link->function == VK_LAYER_LINK_INFO)) {
        link = (VkLayerDeviceCreateInfo *)link->pNext;
    }
    if (!link) return VK_ERROR_INITIALIZATION_FAILED;
    PFN_vkGetInstanceProcAddr gipa = link->u.pLayerInfo->pfnNextGetInstanceProcAddr;
    PFN_vkGetDeviceProcAddr gdpa = link->u.pLayerInfo->pfnNextGetDeviceProcAddr;
    link->u.pLayerInfo = link->u.pLayerInfo->pNext;
    PFN_vkCreateDevice create_device =
        (PFN_vkCreateDevice)gipa(VK_NULL_HANDLE, "vkCreateDevice");
    if (!create_device) return VK_ERROR_INITIALIZATION_FAILED;
    VkResult r = create_device(physicalDevice, pCreateInfo, pAllocator, pDevice);
    if (r != VK_SUCCESS) return r;
    DeviceData *d = (DeviceData *)calloc(1, sizeof(DeviceData));
    if (d) {
        d->key = DISP_KEY(*pDevice);
        d->phys = physicalDevice;
        d->gdpa = gdpa;
        d->create_swapchain =
            (PFN_vkCreateSwapchainKHR)gdpa(*pDevice, "vkCreateSwapchainKHR");
        d->destroy_device = (PFN_vkDestroyDevice)gdpa(*pDevice, "vkDestroyDevice");
        d->next = g_devices;
        g_devices = d;
    }
    return VK_SUCCESS;
}

static VKAPI_ATTR VkResult VKAPI_CALL CreateInstance(
    const VkInstanceCreateInfo *pCreateInfo,
    const VkAllocationCallbacks *pAllocator, VkInstance *pInstance) {
    VkLayerInstanceCreateInfo *link = (VkLayerInstanceCreateInfo *)pCreateInfo->pNext;
    while (link &&
           !(link->sType == VK_STRUCTURE_TYPE_LOADER_INSTANCE_CREATE_INFO &&
             link->function == VK_LAYER_LINK_INFO)) {
        link = (VkLayerInstanceCreateInfo *)link->pNext;
    }
    if (!link) return VK_ERROR_INITIALIZATION_FAILED;
    PFN_vkGetInstanceProcAddr gipa = link->u.pLayerInfo->pfnNextGetInstanceProcAddr;
    link->u.pLayerInfo = link->u.pLayerInfo->pNext;
    g_next_instance_gipa = gipa;
    PFN_vkCreateInstance create_instance =
        (PFN_vkCreateInstance)gipa(VK_NULL_HANDLE, "vkCreateInstance");
    if (!create_instance) return VK_ERROR_INITIALIZATION_FAILED;
    VkResult r = create_instance(pCreateInfo, pAllocator, pInstance);
    if (r == VK_SUCCESS) g_instance = *pInstance;
    return r;
}

static VKAPI_ATTR PFN_vkVoidFunction VKAPI_CALL
GetDeviceProcAddr(VkDevice device, const char *pName) {
    if (!strcmp(pName, "vkGetDeviceProcAddr"))
        return (PFN_vkVoidFunction)GetDeviceProcAddr;
    if (!strcmp(pName, "vkCreateSwapchainKHR"))
        return (PFN_vkVoidFunction)CreateSwapchainKHR;
    if (!strcmp(pName, "vkDestroyDevice"))
        return (PFN_vkVoidFunction)DestroyDevice;
    DeviceData *d = find_device(DISP_KEY(device));
    if (d && d->gdpa) return d->gdpa(device, pName);
    return NULL;
}

static VKAPI_ATTR PFN_vkVoidFunction VKAPI_CALL
GetInstanceProcAddr(VkInstance instance, const char *pName) {
    if (!strcmp(pName, "vkGetInstanceProcAddr"))
        return (PFN_vkVoidFunction)GetInstanceProcAddr;
    if (!strcmp(pName, "vkCreateInstance"))
        return (PFN_vkVoidFunction)CreateInstance;
    if (!strcmp(pName, "vkCreateDevice"))
        return (PFN_vkVoidFunction)CreateDevice;
    if (!strcmp(pName, "vkGetDeviceProcAddr"))
        return (PFN_vkVoidFunction)GetDeviceProcAddr;
    if (!strcmp(pName, "vkCreateSwapchainKHR"))
        return (PFN_vkVoidFunction)CreateSwapchainKHR;
    if (!strcmp(pName, "vkDestroyDevice"))
        return (PFN_vkVoidFunction)DestroyDevice;
    if (g_next_instance_gipa) return g_next_instance_gipa(instance, pName);
    return NULL;
}

VL_EXPORT VKAPI_ATTR VkResult VKAPI_CALL
vkNegotiateLoaderLayerInterfaceVersion(VkNegotiateLayerInterface *pVersionStruct) {
    if (pVersionStruct->loaderLayerInterfaceVersion > 2)
        pVersionStruct->loaderLayerInterfaceVersion = 2;
    pVersionStruct->pfnGetInstanceProcAddr = GetInstanceProcAddr;
    pVersionStruct->pfnGetDeviceProcAddr = GetDeviceProcAddr;
    pVersionStruct->pfnGetPhysicalDeviceProcAddr = NULL;
    return VK_SUCCESS;
}
